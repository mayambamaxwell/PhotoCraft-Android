//! Movable selection launcher, through existing commands.
use photocraft_ui_egui::{menus, PhotocraftApp};
use serde_json::json;
pub struct SelectionBar {
    pub radius: u32,
    pub rect: Option<egui::Rect>,
}
impl Default for SelectionBar {
    fn default() -> Self {
        Self { radius: 1, rect: None }
    }
}
impl SelectionBar {
    pub fn ui(&mut self, app: &mut PhotocraftApp, ui: &mut egui::Ui) {
        if app.session.active().is_none_or(|d| d.doc.selection.is_none()) {
            self.rect = None;
            return;
        }
        let mut pos = egui::pos2(100., 150.);
        if let (Some(xf), Some(b)) =
            (photocraft_ui_egui::canvas::ViewXform::active(app), app.session.active().and_then(|d| d.doc.selection.as_ref()).map(|s| s.content_bounds()))
        {
            pos = xf.to_screen(b.x0 as f32, b.y1 as f32) + egui::vec2(0., 20.);
            pos.x = pos.x.clamp(10., (ui.ctx().content_rect().right() - 700.).max(10.));
            pos.y = pos.y.clamp(40., (ui.ctx().content_rect().bottom() - 120.).max(40.));
        }
        let shown = egui::Area::new(egui::Id::new("selection-launcher")).order(egui::Order::Foreground).default_pos(pos).movable(true).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(540.);
                ui.horizontal_wrapped(|ui| {
                    ui.add(photocraft_ui_egui::icons::image("move", 18., photocraft_ui_egui::theme::Tokens::get(ui.ctx()).text_faint))
                        .on_hover_text("Drag to move selection actions");
                    ui.label("Selection");
                    if app.ui.transform.is_some() {
                        if ui.add_sized([96., 36.], egui::Button::new("Apply selection transform")).clicked() {
                            photocraft_ui_egui::transform_tool::commit(app);
                        }
                        if ui.add_sized([96., 36.], egui::Button::new("Cancel selection transform")).clicked() {
                            photocraft_ui_egui::transform_tool::cancel(app);
                        }
                        ui.label("Drag corners to scale; drag outside a corner to rotate.");
                        return;
                    }
                    ui.add(egui::DragValue::new(&mut self.radius).range(1..=500).prefix("Size: ").suffix(" px"));
                    for (label, id, icon) in [
                        ("Deselect", "select.deselect", "square-dashed"),
                        ("Invert selection", "select.inverse", "squares-subtract"),
                        ("Expand", "select.modify.expand", "scaling"),
                        ("Shrink", "select.modify.contract", "scan"),
                        ("Feather", "select.modify.feather", "tablet-feather"),
                        ("Transform selection", "select.transformSelection", "square-dashed-mouse-pointer"),
                        ("Delete selected area", "edit.clear", "trash"),
                        ("Cut to new layer", "layer.new.layerViaCut", "scissors"),
                        ("Copy to new layer", "layer.new.layerViaCopy", "copy"),
                        ("Scale / rotate", "edit.freeTransform", "rotate-cw"),
                        ("Free form", "edit.transform.distort", "move"),
                    ] {
                        if ui.add_enabled_ui(menus::is_enabled(app, id), |ui| photocraft_ui_egui::icons::button(ui, icon, 36., false, label)).inner.clicked() {
                            let params = if id.starts_with("select.modify.") { json!({"radius":self.radius.clamp(1,500)}) } else { json!({}) };
                            if let Err(e) = menus::invoke(app, ui.ctx(), id, params) {
                                app.ui.status = e;
                            }
                        }
                    }
                });
            });
        });
        self.rect = Some(shown.response.rect);
    }
}
