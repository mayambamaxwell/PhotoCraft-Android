//! Persisted tablet settings and movable quick actions.
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, PoisonError};
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub pen_only: bool,
    pub palm: bool,
    pub touch_size: f32,
    pub quick: Vec<String>,
    pub show_quick: bool,
    pub show_reference: bool,
    #[serde(skip)]
    pub changed: bool,
    #[serde(skip)]
    pub restore: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            pen_only: true,
            palm: true,
            touch_size: 36.,
            quick: vec![
                "edit.undo".into(),
                "edit.redo".into(),
                "file.save".into(),
                "file.open".into(),
                "window.panel.timeline".into(),
                "view.fitOnScreen".into(),
            ],
            show_quick: true,
            show_reference: false,
            changed: false,
            restore: false,
        }
    }
}
pub type Shared = Arc<Mutex<Settings>>;
pub const COMMANDS: &[(&str, &str, &str)] = &[
    ("edit.undo", "Undo", "undo-2"),
    ("edit.redo", "Redo", "redo-2"),
    ("file.save", "Save", "tablet-save"),
    ("file.open", "Open", "folder-open"),
    ("file.placeEmbedded", "Import image", "image"),
    ("window.panel.timeline", "Animation", "play"),
    ("view.fitOnScreen", "Fit canvas", "scan"),
    ("view.resetView", "Reset rotation", "rotate-cw"),
    ("view.flipHorizontal", "Flip view", "arrow-left-right"),
    ("window.panel.layers", "Layers", "layers"),
    ("window.panel.color", "Colour", "palette"),
];
pub fn preferences(ui: &mut egui::Ui, shared: &Shared) {
    let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let before = serde_json::to_vec(&*s).unwrap_or_default();
    ui.checkbox(&mut s.pen_only, "Only draw with pen");
    ui.checkbox(&mut s.palm, "Reject palm near pen");
    ui.add(egui::Slider::new(&mut s.touch_size, 32.0..=52.0).text("Touch control size"));
    ui.checkbox(&mut s.show_quick, "Show quick actions");
    ui.checkbox(&mut s.show_reference, "Show reference image panel");
    ui.collapsing("Customize quick actions", |ui| {
        for (id, label, _) in COMMANDS {
            let mut on = s.quick.iter().any(|x| x == id);
            if ui.checkbox(&mut on, *label).changed() {
                s.quick.retain(|x| x != id);
                if on {
                    s.quick.push((*id).into());
                }
            }
        }
    });
    if ui.button("Restore recovery workspace").clicked() {
        s.restore = true;
    }
    if before != serde_json::to_vec(&*s).unwrap_or_default() {
        s.changed = true;
    }
}
pub fn quick_actions(app: &mut photocraft_ui_egui::PhotocraftApp, ctx: &egui::Context, shared: &Shared) {
    let settings = shared.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if !settings.show_quick || app.session.active().is_none() {
        return;
    }
    let mut chosen = None;
    egui::Area::new(egui::Id::new("tablet-quick-actions"))
        .order(egui::Order::Foreground)
        .default_pos(egui::pos2(70., ctx.content_rect().bottom() - 90.))
        .movable(true)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(photocraft_ui_egui::icons::image("move", 18., photocraft_ui_egui::theme::Tokens::get(ui.ctx()).text_faint))
                        .on_hover_text("Drag to move quick actions");
                    for (id, label, icon) in COMMANDS.iter().filter(|(id, _, _)| settings.quick.iter().any(|x| x == id)) {
                        if photocraft_ui_egui::icons::button(ui, icon, settings.touch_size, false, label).clicked() {
                            chosen = Some(*id);
                        }
                    }
                    if photocraft_ui_egui::icons::button(ui, "image", settings.touch_size, settings.show_reference, "Reference images").clicked() {
                        let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
                        s.show_reference = !settings.show_reference;
                        s.changed = true;
                    }
                    if photocraft_ui_egui::icons::button(ui, "tablet-share", settings.touch_size, false, "Share image").clicked() {
                        chosen = Some("android.share");
                    }
                });
            });
        });
    if let Some(id) = chosen {
        if id == "android.share" {
            #[cfg(target_os = "android")]
            if let Err(e) = crate::platform::share(app) {
                app.ui.status = e;
            }
        } else if let Err(e) = photocraft_ui_egui::menus::invoke(app, ctx, id, serde_json::json!({})) {
            app.ui.status = e;
        }
    }
}
