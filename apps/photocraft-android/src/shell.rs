use photocraft_ui_egui::{FileDialogAnswer, FileDialogReply, FileDialogRequest, PhotocraftApp};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, PoisonError,
};
pub type Dialog = Arc<Mutex<Option<(FileDialogRequest, FileDialogReply)>>>;
pub struct Shell {
    pub app: PhotocraftApp,
    pub dialogs: Dialog,
    pub folder: std::path::PathBuf,
    pub name: String,
    pub error: String,
    pub in_range: Arc<AtomicBool>,
    pub palm: Arc<AtomicBool>,
    pub pen_only: bool,
    pub gestures: crate::gestures::Gestures,
    pub tablet: crate::tablet::Shared,
    pub selection: crate::selection::SelectionBar,
    pub timelapse: crate::timelapse::Recorder,
    pub workspace: Option<crate::workspace::Workspace>,
}
impl eframe::App for Shell {
    // PhotoCraft completes font setup and advances editor state here, before drawing.
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let pointer_modifiers = input
            .events
            .iter()
            .rev()
            .find_map(|e| match e {
                egui::Event::PointerButton { modifiers, .. } | egui::Event::Key { modifiers, .. } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or_else(|| ctx.input(|i| i.modifiers));
        let touches = input.events.iter().any(|e| matches!(e, egui::Event::Touch { .. }));
        let xf = photocraft_ui_egui::canvas::ViewXform::active(&self.app);
        let overlay = input.events.iter().any(|e| match e {
            egui::Event::Touch { pos, .. } => {
                self.selection.rect.is_some_and(|r| r.contains(*pos)) || ctx.layer_id_at(*pos).is_some_and(|l| l.order > egui::Order::Background)
            }
            _ => false,
        });
        let canvas = if self.app.ui.dialogs.is_empty() && !overlay { xf.map(|x| x.rect) } else { None };
        let reference = ctx.data(|d| d.get_temp::<(egui::Rect, u64)>(egui::Id::new("reference-pick")));
        self.gestures.reference_rect = reference.map(|r| r.0);
        let change = self.gestures.filter(&mut input.events, canvas, self.pen_only, self.palm.load(Ordering::Relaxed), self.in_range.load(Ordering::Relaxed));
        if self.gestures.holding() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        if let Some(pos) = self.gestures.take_pick() {
            let sample = if let Some((rect, id)) = reference.filter(|r| r.0.contains(pos)) {
                self.app.session.documents().iter().find(|d| d.doc.id.0 == id).map(|d| {
                    let x = (pos.x - rect.left()) / rect.width() * d.doc.size.width as f32;
                    let y = (pos.y - rect.top()) / rect.height() * d.doc.size.height as f32;
                    photocraft_engine::sample_cmds::sample_color(&d.doc, None, x as f64, y as f64, 1, photocraft_engine::sample_cmds::SampleLayers::All)
                })
            } else {
                xf.and_then(|xf| {
                    self.app.session.active().map(|d| {
                        let p = xf.to_doc(pos);
                        photocraft_engine::sample_cmds::sample_color(&d.doc, None, p[0], p[1], 1, photocraft_engine::sample_cmds::SampleLayers::All)
                    })
                })
            };
            if let Some(color) = sample.filter(|c| c[3] > 0.) {
                let _ = self.app.run("tools.setColors", serde_json::json!({"foreground":[color[0],color[1],color[2],1.]}));
                self.app.ui.status = "Colour picked".into();
            }
        }
        if touches {
            for event in &mut input.events {
                if let egui::Event::PointerButton { modifiers, .. } = event {
                    *modifiers = pointer_modifiers;
                }
            }
        }
        if !input.focused || input.events.iter().any(|e| matches!(e, egui::Event::WindowFocused(false))) {
            self.in_range.store(false, Ordering::Relaxed);
            self.app.stylus.feed.set(None);
            self.gestures.reset();
            self.app.ui.timeline.playing = false;
            if let Some(w) = self.workspace.as_mut() {
                w.tick(&mut self.app, self.pen_only);
            }
            let pos = egui::pos2(-10000.0, -10000.0);
            input.events.push(egui::Event::PointerMoved(pos));
            input.events.push(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
            input.events.push(egui::Event::PointerGone);
        }
        if let (Some(delta), Some(xf), Some(index)) = (change.filter(|_| input.focused), xf, self.app.session.active_index()) {
            let anchor = xf.to_doc(delta.before);
            let ppp = self.app.canvas_ppp();
            if let Some(view) = self.app.ui.views.get_mut(index) {
                view.zoom = photocraft_ui_egui::zoom_levels::clamp(view.zoom * delta.scale, view.doc_size);
                view.rotation = (view.rotation + (if xf.flip { -delta.radians } else { delta.radians }).to_degrees() + 180.0).rem_euclid(360.0) - 180.0;
                view.fit_pending = false;
                view.fill_pending = false;
                let mut next = xf;
                next.zoom = view.zoom / ppp;
                next.rotation = view.rotation;
                let offset = next.unmap_vec(delta.after - next.rect.center()) / next.zoom;
                view.center = [anchor[0] as f32 - offset.x, anchor[1] as f32 - offset.y];
                ctx.request_repaint();
            }
        }
        if input.focused {
            if let Some(tap) = self.gestures.take_tap() {
                let id = match tap {
                    crate::gestures::Tap::Undo => "edit.undo",
                    crate::gestures::Tap::Redo => "edit.redo",
                };
                if photocraft_ui_egui::menus::is_enabled(&self.app, id) {
                    if let Err(error) = photocraft_ui_egui::menus::invoke(&mut self.app, ctx, id, serde_json::json!({})) {
                        self.app.ui.status = error;
                    }
                }
            }
        }
        self.app.raw_input_hook(ctx, input);
    }
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        #[cfg(target_os = "android")]
        if let Some(e) = crate::platform::take_error() {
            self.app.ui.status = e;
            self.app.ui.status_error = true;
        }
        {
            let mut settings = self.tablet.lock().unwrap_or_else(PoisonError::into_inner);
            ui.spacing_mut().interact_size = egui::vec2(settings.touch_size, settings.touch_size);
            ui.ctx().style_mut_of(ui.ctx().theme(), |style| style.spacing.interact_size = egui::vec2(settings.touch_size, settings.touch_size));
            if settings.changed {
                self.pen_only = settings.pen_only;
                self.palm.store(settings.palm, Ordering::Relaxed);
                if let Ok(bytes) = serde_json::to_vec(&*settings) {
                    let root = self.workspace.as_ref().and_then(|w| w.path().parent()).unwrap_or(&self.folder);
                    if let Err(e) = crate::storage::write_atomic(&root.join("tablet.json").to_string_lossy(), &bytes) {
                        self.app.ui.status = e;
                    }
                }
                settings.changed = false;
            }
            if settings.restore {
                settings.restore = false;
                if let Err(e) = self.app.recover_workspace() {
                    self.app.ui.status = e;
                }
            }
        }
        if self.workspace.as_ref().is_some_and(|w| w.recovery_offer) {
            let mut answer = None;
            egui::Window::new("Restore your workspace?").collapsible(false).show(ui.ctx(), |ui| {
                ui.label("Saved recovery checkpoints are available. Restore your open projects and canvas views?");
                ui.horizontal(|ui| {
                    if ui.button("Restore workspace").clicked() {
                        answer = Some(true);
                    }
                    if ui.button("Not now").clicked() {
                        answer = Some(false);
                    }
                });
            });
            if let Some(restore) = answer {
                if let Some(w) = self.workspace.as_mut() {
                    w.recovery_offer = false;
                }
                if restore {
                    if let Err(e) = self.app.recover_workspace() {
                        self.app.ui.status = e;
                    }
                }
            }
        }
        self.app.ui(ui, frame);
        self.timelapse.tick(&mut self.app, ui.ctx());
        self.selection.ui(&mut self.app, ui);
        crate::tablet::quick_actions(&mut self.app, ui.ctx(), &self.tablet);
        crate::reference::panel(&mut self.app, ui.ctx(), &self.tablet);
        if let Some(workspace) = self.workspace.as_mut() {
            workspace.tick(&mut self.app, self.pen_only);
            let status = if let Some(e) = &workspace.last_error {
                format!("Recovery save failed: {e}")
            } else if workspace.saving {
                "Saving recovery checkpoint…".into()
            } else if let Some(t) = workspace.last_completed {
                format!("Recovery saved {}s ago", t.elapsed().unwrap_or_default().as_secs())
            } else {
                "Recovery ready".into()
            };
            egui::Area::new(egui::Id::new("recovery-status")).anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(55., -32.)).interactable(false).show(
                ui.ctx(),
                |ui| {
                    ui.label(status);
                },
            );
            if let Some(delay) = workspace.wake_after {
                ui.ctx().request_repaint_after(delay);
            }
        }

        let pending = self.dialogs.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some((request, reply)) = pending {
            if self.name.is_empty() {
                if let FileDialogRequest::Save { suggested } = &request {
                    self.name = std::path::Path::new(suggested).file_name().and_then(|s| s.to_str()).unwrap_or("Untitled.pcraft").to_owned();
                }
            }
            #[cfg(target_os = "android")]
            if crate::platform::picker_active() {
                match crate::platform::poll_picker() {
                    Ok(Some(answer)) => crate::platform::send_reply(reply, answer),
                    Ok(None) => {
                        *self.dialogs.lock().unwrap_or_else(PoisonError::into_inner) = Some((request, reply));
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    Err(e) => {
                        self.app.ui.status = e;
                        reply.send(None);
                    }
                }
                return;
            }
            let mut answer = None;
            let mut finished = false;
            egui::Window::new("Android files").collapsible(false).show(ui.ctx(), |ui| {
                ui.label("Android file picker could not open. App-owned files are available below.");
                ui.label(self.folder.display().to_string());
                if ui.button("Parent folder").clicked() {
                    if let Some(parent) = self.folder.parent() {
                        self.folder = parent.to_owned();
                    }
                }

                egui::ScrollArea::vertical().id_salt("folders").max_height(160.0).show(ui, |ui| {
                    if let Ok(entries) = std::fs::read_dir(&self.folder) {
                        for entry in entries.take(200).flatten() {
                            let path = entry.path();
                            if path.is_dir() && ui.button(format!("Folder: {}", entry.file_name().to_string_lossy())).clicked() {
                                self.folder = path;
                            }
                        }
                    }
                });
                match &request {
                    FileDialogRequest::Save { .. } => {
                        ui.text_edit_singleline(&mut self.name);
                        if ui.button("Save").clicked() {
                            if self.name.is_empty() || self.name.contains(['/', '\\']) || self.name == ".." {
                                self.error = "Enter a file name without folder separators".into();
                            } else {
                                answer = Some(FileDialogAnswer::SaveTo(self.folder.join(&self.name).display().to_string()));
                                finished = true;
                            }
                        }
                    }
                    FileDialogRequest::Open { .. } => {
                        egui::ScrollArea::vertical().id_salt("files").max_height(280.0).show(ui, |ui| match std::fs::read_dir(&self.folder) {
                            Ok(entries) => {
                                for entry in entries.take(200).flatten() {
                                    let path = entry.path();
                                    if path.is_file() && ui.button(entry.file_name().to_string_lossy()).clicked() {
                                        match std::fs::metadata(&path) {
                                            Ok(meta) if meta.len() <= 64 * 1024 * 1024 => {
                                                match std::fs::File::open(&path)
                                                    .map_err(|e| e.to_string())
                                                    .and_then(|file| crate::storage::read_bounded(file, 64 * 1024 * 1024))
                                                {
                                                    Ok(bytes) => {
                                                        answer = Some(FileDialogAnswer::ContentsMany(vec![(path.display().to_string(), bytes)]));
                                                        finished = true;
                                                    }
                                                    Err(e) => self.error = e.to_string(),
                                                }
                                            }
                                            _ => self.error = "Files are limited to 64 MB in this preview".into(),
                                        }
                                    }
                                }
                            }
                            Err(e) => self.error = e.to_string(),
                        });
                    }
                }
                if !self.error.is_empty() {
                    ui.label(&self.error);
                }
                if ui.button("Cancel").clicked() {
                    finished = true;
                }
            });
            if finished {
                reply.send(answer);
                self.name.clear();
                self.error.clear();
            } else {
                *self.dialogs.lock().unwrap_or_else(PoisonError::into_inner) = Some((request, reply));
            }
        }
    }
}
