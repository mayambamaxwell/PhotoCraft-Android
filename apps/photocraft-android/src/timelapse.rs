//! Bounded per-document progress frames, captured on a worker after completed edits.
use photocraft_ui_egui::PhotocraftApp;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
enum Task {
    Capture(PathBuf, photocraft_doc::Document),
    Export(PathBuf, photocraft_doc::Document, i32),
}
enum Completed {
    CaptureError(String),
    Export(Result<String, String>),
}
pub struct Recorder {
    pub requested: Arc<AtomicBool>,
    root: PathBuf,
    sender: mpsc::SyncSender<Task>,
    results: mpsc::Receiver<Completed>,
    seen: BTreeMap<u64, u64>,
    next: Instant,
    open: bool,
    seconds: i32,
    enabled: bool,
    exporting: bool,
    status: String,
}
impl Recorder {
    pub fn new(root: PathBuf) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Task>(1);
        let (result_sender, results) = mpsc::channel();
        let worker = std::thread::Builder::new().name("photocraft-timelapse".into()).spawn(move || {
            while let Ok(task) = receiver.recv() {
                match task {
                    Task::Capture(folder, doc) => {
                        if let Err(e) = capture(&folder, &doc) {
                            let _ = result_sender.send(Completed::CaptureError(e));
                        }
                    }
                    Task::Export(folder, doc, seconds) => {
                        let result = capture(&folder, &doc).and_then(|_| {
                            #[cfg(target_os = "android")]
                            {
                                crate::platform::timelapse(&folder.to_string_lossy(), seconds).map(|_| "Timelapse saved to Gallery → PhotoCraft".to_string())
                            }
                            #[cfg(not(target_os = "android"))]
                            {
                                let _ = seconds;
                                Err("Video encoding requires Android".into())
                            }
                        });
                        let _ = result_sender.send(Completed::Export(result));
                    }
                }
            }
        });
        let status = worker.err().map_or(String::new(), |e| format!("Recording unavailable: {e}"));
        Self {
            requested: Arc::new(AtomicBool::new(false)),
            root: root.join("timelapse"),
            sender,
            results,
            seen: BTreeMap::new(),
            next: Instant::now(),
            open: false,
            seconds: 30,
            enabled: true,
            exporting: false,
            status,
        }
    }
    pub fn tick(&mut self, app: &mut PhotocraftApp, ctx: &egui::Context) {
        while let Ok(result) = self.results.try_recv() {
            match result {
                Completed::CaptureError(e) => {
                    self.status = format!("Recording paused: {e}");
                    self.enabled = false;
                    app.ui.status = self.status.clone();
                }
                Completed::Export(result) => {
                    self.exporting = false;
                    self.status = result.unwrap_or_else(|e| e);
                    app.ui.status = self.status.clone();
                }
            }
        }

        if self.requested.swap(false, Ordering::Relaxed) {
            self.open = true;
        }
        if self.enabled && !self.exporting && !ctx.input(|i| i.pointer.any_down()) && Instant::now() >= self.next {
            if let Some(d) = app.session.active() {
                let id = d.doc.id.0;
                let rev = d.revision;
                if self.seen.get(&id) != Some(&rev) && self.sender.try_send(Task::Capture(self.root.join(id.to_string()), (*d.doc).clone())).is_ok() {
                    self.seen.insert(id, rev);
                }
            }
            self.next = Instant::now() + Duration::from_secs(3);
        }
        if self.enabled || self.exporting {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
        if !self.open {
            return;
        }
        let mut open = true;
        egui::Window::new("Export timelapse video").open(&mut open).collapsible(false).show(ctx, |ui| {
            ui.checkbox(&mut self.enabled, "Record drawing progress");
            ui.label("Records completed edits, up to once every 3 seconds. Older frames are thinned as the recording grows.");
            ui.label("MP4 • 512 × 512 • 24 fps • artwork fitted without cropping");
            ui.horizontal_wrapped(|ui| {
                for seconds in [10, 30, 60, 120, 300] {
                    ui.selectable_value(&mut self.seconds, seconds, format!("{seconds}s"));
                }
            });
            ui.label("Videos are saved to Gallery → PhotoCraft. Recording begins with this update; earlier edits cannot be reconstructed.");
            if ui.add_enabled(!self.exporting && app.session.active().is_some(), egui::Button::new("Generate and save video to Gallery")).clicked() {
                if let Some(d) = app.session.active() {
                    let folder = self.root.join(d.doc.id.0.to_string());
                    match self.sender.try_send(Task::Export(folder, (*d.doc).clone(), self.seconds)) {
                        Ok(()) => {
                            self.exporting = true;
                            self.status = "Generating video… Keep PhotoCraft open until it completes.".into();
                        }
                        Err(_) => self.status = "Finishing a recorded frame. Please try again in a moment.".into(),
                    }
                }
            }
            ui.label(&self.status);
        });
        self.open = open;
    }
}
fn capture(folder: &std::path::Path, doc: &photocraft_doc::Document) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let mut files: Vec<_> =
        std::fs::read_dir(folder).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "png")).collect();
    files.sort();
    let index = files.last().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0).saturating_add(1);
    let bytes: u64 = files.iter().filter_map(|p| p.metadata().ok()).map(|m| m.len()).sum();
    if files.len() >= 300 || bytes > 48 * 1024 * 1024 {
        for (i, p) in files.iter().enumerate() {
            if i % 2 == 1 {
                std::fs::remove_file(p).map_err(|e| e.to_string())?;
            }
        }
    }
    let image = photocraft_compose::thumbnail(doc, 512);
    let image =
        photocraft_codecs::Image::from_raw(image.width, image.height, photocraft_codecs::ChannelLayout::Rgba, photocraft_codecs::SampleType::U8, image.pixels)
            .map_err(|e| e.to_string())?;
    let bytes = photocraft_codecs::encode(&image, photocraft_codecs::Format::Png, &Default::default()).map_err(|e| e.to_string())?;
    crate::storage::write_atomic(&folder.join(format!("{index:020}.png")).to_string_lossy(), &bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_captures_changed_document() {
        let root = std::env::temp_dir().join(format!("photocraft-record-test-{}", std::process::id()));
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.run("file.new", serde_json::json!({"width":16,"height":12})).unwrap();
        let doc = app.session.active().unwrap().doc.clone();
        capture(&root, &doc).unwrap();
        capture(&root, &doc).unwrap();
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
        std::fs::remove_dir_all(root).unwrap();
    }
}
