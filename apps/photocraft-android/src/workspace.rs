//! Incremental background checkpoints survive Android process death, including saved open files.
use photocraft_format::RecoveryStore;
use photocraft_ui_egui::{state::View, PhotocraftApp, Recoverable, Services};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Metadata {
    views: BTreeMap<String, View>,
    active: Option<String>,
    pen_only: bool,
}
impl Default for Metadata {
    fn default() -> Self {
        Self { views: BTreeMap::new(), active: None, pen_only: true }
    }
}
struct State {
    store: RecoveryStore,
    keys: BTreeMap<u64, String>,
    restored: BTreeMap<u64, String>,
}
pub struct Workspace {
    pending: BTreeSet<u64>,
    metadata_jobs: usize,
    pub wake_after: Option<Duration>,
    wanted_active: Option<String>,
    results: std::sync::mpsc::Receiver<Result<(), String>>,
    pub recovery_offer: bool,
    pub last_completed: Option<std::time::SystemTime>,
    pub saving: bool,
    pub last_error: Option<String>,
    state: Arc<Mutex<State>>,
    seen: BTreeMap<u64, (u64, Option<String>)>,
    meta: Metadata,
    metadata_path: PathBuf,
    next_meta: Instant,
    sender: std::sync::mpsc::SyncSender<Vec<u8>>,
    last_meta: Vec<u8>,
}
impl Workspace {
    pub fn new(folder: PathBuf) -> (Self, Services) {
        let dir = folder.join("workspace");
        let metadata_path = folder.join("workspace-view.json");
        let mut meta: Metadata = std::fs::File::open(&metadata_path)
            .ok()
            .and_then(|f| crate::storage::read_bounded(f, 1 << 20).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if !metadata_path.exists() {
            meta.pen_only = true;
        }
        let state = Arc::new(Mutex::new(State { store: RecoveryStore::new(&dir), keys: BTreeMap::new(), restored: BTreeMap::new() }));
        let adopted = state.clone();
        let recovery_dir = dir.clone();
        let services = Services {
            recover: Some(Box::new(move || {
                photocraft_format::list_recovery(&recovery_dir)
                    .into_iter()
                    .map(|entry| Recoverable {
                        key: entry.info.key.clone(),
                        name: entry.info.document_name.clone(),
                        path: entry.info.original_path.clone(),
                        load: Box::new(move || photocraft_format::recover(&entry).map_err(|e| e.to_string())),
                    })
                    .collect()
            })),
            adopt_autosave: Some(Box::new(move |id, key| {
                let mut s = adopted.lock().unwrap_or_else(PoisonError::into_inner);
                s.store.adopt(id, key);
                s.keys.insert(id, key.into());
                s.restored.insert(id, key.into());
            })),
            ..Default::default()
        };
        let (result_sender, results) = std::sync::mpsc::channel();
        let wanted_active = meta.active.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);
        let path = metadata_path.clone();
        let writer = std::thread::Builder::new().name("photocraft-workspace".into()).spawn(move || {
            while let Ok(bytes) = receiver.recv() {
                let result = crate::storage::write_atomic(&path.to_string_lossy(), &bytes);
                let _ = result_sender.send(result);
            }
        });
        (
            Self {
                pending: BTreeSet::new(),
                metadata_jobs: 0,
                wake_after: None,
                wanted_active,
                results,
                recovery_offer: !photocraft_format::list_recovery(&folder.join("workspace")).is_empty(),
                last_completed: None,
                saving: false,
                last_error: writer.err().map(|e| e.to_string()),
                state,
                seen: BTreeMap::new(),
                meta,
                metadata_path,
                next_meta: Instant::now(),
                sender,
                last_meta: Vec::new(),
            },
            services,
        )
    }
    pub fn pen_only(&self) -> bool {
        self.meta.pen_only
    }
    pub fn tick(&mut self, app: &mut PhotocraftApp, pen_only: bool) {
        let mut s = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let restored = std::mem::take(&mut s.restored);
        let recovering = !restored.is_empty();
        for (id, key) in restored {
            if let Some(i) = app.session.documents().iter().position(|d| d.doc.id.0 == id) {
                if let (Some(view), Some(slot)) = (self.meta.views.get(&key), app.ui.views.get_mut(i)) {
                    if view.zoom.is_finite() && view.zoom > 0.0 && view.center.iter().all(|x| x.is_finite()) && view.rotation.is_finite() {
                        *slot = view.clone();
                    }
                }
                if self.wanted_active.as_ref() == Some(&key) {
                    let _ = app.run("document.activate", serde_json::json!({"document":i}));
                }
            }
        }
        if recovering {
            if let Some(wanted) = self.wanted_active.as_ref() {
                if let Some(i) = app.session.documents().iter().position(|d| s.keys.get(&d.doc.id.0) == Some(wanted)) {
                    app.session.set_active(i);
                }
            }
        }
        let live: Vec<u64> = app.session.documents().iter().map(|d| d.doc.id.0).collect();
        let closed: Vec<_> = self.seen.keys().filter(|id| !live.contains(id)).copied().collect();
        for id in closed {
            if let Err(e) = s.store.discard(id) {
                log::error!("Workspace close checkpoint: {e}");
            }
            if let Some(key) = s.keys.remove(&id) {
                self.meta.views.remove(&key);
            }
            self.seen.remove(&id);
            self.pending.remove(&id);
        }
        for (i, d) in app.session.documents().iter().enumerate() {
            let key = s
                .keys
                .entry(d.doc.id.0)
                .or_insert_with(|| {
                    format!("android-doc-{}-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos(), d.doc.id.0)
                })
                .clone();
            s.store.adopt(d.doc.id.0, &key);
            let signature = (d.revision, d.path.clone());
            if !app.ui.timeline.playing && self.seen.get(&d.doc.id.0) != Some(&signature) {
                match s.store.autosave_checked(&d.doc, d.revision, d.path.clone()) {
                    Ok(()) => {
                        self.seen.insert(d.doc.id.0, signature);
                        self.pending.insert(d.doc.id.0);
                    }
                    Err(e) => {
                        self.last_error = Some(e.to_string());
                        log::error!("Workspace enqueue: {e}");
                    }
                }
            }
            if let Some(v) = app.ui.views.get(i) {
                self.meta.views.insert(key.clone(), v.clone());
            }
            if Some(i) == app.session.active_index() {
                self.meta.active = Some(key);
            }
        }
        for (id, _rev, result) in s.store.take_completed() {
            self.pending.remove(&id);
            if result.is_ok() {
                self.last_completed = Some(std::time::SystemTime::now());
                self.last_error = None;
            }
            if let Err(e) = result {
                self.seen.remove(&id);
                self.last_error = Some(e.clone());
                log::error!("Workspace document checkpoint: {e}");
            }
        }
        if app.session.active_index().is_none() {
            self.meta.active = None;
        }
        drop(s);
        for result in self.results.try_iter() {
            self.metadata_jobs = self.metadata_jobs.saturating_sub(1);
            if let Err(e) = result {
                self.last_meta.clear();
                self.last_error = Some(e);
            }
        }
        self.meta.pen_only = pen_only;
        let now = Instant::now();
        let bytes = match serde_json::to_vec(&self.meta) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.last_error = Some(format!("Cannot checkpoint workspace settings: {error}"));
                return;
            }
        };
        let dirty = bytes != self.last_meta;
        self.wake_after = None;
        if dirty && now >= self.next_meta {
            match self.sender.try_send(bytes.clone()) {
                Ok(()) => {
                    self.last_meta = bytes;
                    self.metadata_jobs = self.metadata_jobs.saturating_add(1);
                }
                Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => self.last_error = Some("Workspace settings writer stopped".into()),
            }
            self.next_meta = now + Duration::from_millis(250);
        }
        if dirty {
            self.wake_after = Some(self.next_meta.saturating_duration_since(now));
        }
        self.saving = !self.pending.is_empty() || self.metadata_jobs > 0;
        if self.saving {
            self.wake_after = Some(Duration::from_millis(100));
        }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.metadata_path
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoints_restore_pixels_view_and_pen_option_after_restart() {
        let folder =
            std::env::temp_dir().join(format!("photocraft-restart-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&folder).unwrap();
        let (mut workspace, services) = Workspace::new(folder.clone());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services);
        app.run("file.new", serde_json::json!({"width":16,"height":16,"background":"white"})).unwrap();
        app.run("edit.fill", serde_json::json!({"color":"#ff0000"})).unwrap();
        let state = app.session.active_mut().unwrap();
        state.path = Some("/test/saved.pcraft".into());
        state.saved_revision = state.revision;
        let expected = photocraft_io::export(&state.doc, "test.png", &Default::default()).unwrap().bytes;
        app.sync_views();
        app.ui.views[0].zoom = 2.0;
        app.ui.views[0].rotation = 30.0;
        app.ui.views[0].center = [3., 4.];
        workspace.tick(&mut app, false);
        let start = Instant::now();
        while (workspace.last_completed.is_none() || !workspace.path().exists()) && start.elapsed() < Duration::from_secs(5) {
            workspace.tick(&mut app, false);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(workspace.last_completed.is_some());
        drop(app);
        drop(workspace);
        let (mut workspace, mut services) = Workspace::new(folder.clone());
        assert!(workspace.recovery_offer);
        assert!(!workspace.pen_only());
        let entries = services.recover.as_mut().unwrap()();
        assert_eq!(entries.len(), 1);
        let entry = entries.into_iter().next().unwrap();
        let key = entry.key.clone();
        let doc = (entry.load)().unwrap();
        assert_eq!(entry.path.as_deref(), Some("/test/saved.pcraft"));
        assert_eq!(photocraft_io::export(&doc, "test.png", &Default::default()).unwrap().bytes, expected);
        assert_eq!(doc.bounds().width(), 16);
        assert_eq!(doc.bounds().height(), 16);
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Services::default());
        app.session.add_document(doc, entry.path);
        app.sync_views();
        services.adopt_autosave.as_mut().unwrap()(app.session.active().unwrap().doc.id.0, &key);
        workspace.tick(&mut app, false);
        assert_eq!(app.ui.views[0].rotation, 30.0);
        assert_eq!(app.ui.views[0].zoom, 2.0);
        assert_eq!(app.ui.views[0].center, [3., 4.]);
        app.session.execute("file.close", serde_json::json!({})).unwrap();
        workspace.tick(&mut app, false);
        assert!(photocraft_format::list_recovery(&folder.join("workspace")).is_empty());
        drop(app);
        drop(workspace);
        drop(services);
        let _ = std::fs::remove_dir_all(folder);
    }
}
