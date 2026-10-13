//! Pinned reference documents and reference-layer thumbnails.
use photocraft_ui_egui::PhotocraftApp;
use std::sync::PoisonError;
pub fn panel(app: &mut PhotocraftApp, ctx: &egui::Context, settings: &crate::tablet::Shared) {
    ctx.data_mut(|d| d.remove::<(egui::Rect, u64)>(egui::Id::new("reference-pick")));
    if !settings.lock().unwrap_or_else(PoisonError::into_inner).show_reference {
        return;
    }
    let key = egui::Id::new("reference-document");
    let mut pinned = ctx.data(|d| d.get_temp::<u64>(key)).or_else(|| app.session.active().map(|d| d.doc.id.0));
    let docs: Vec<_> = app.session.documents().iter().map(|d| (d.doc.id.0, d.doc.name.clone(), d.doc.size)).collect();
    let mut open = true;
    let mut import = false;
    egui::Window::new("Reference images").open(&mut open).default_pos(egui::pos2(80., 120.)).default_size(egui::vec2(300., 330.)).show(ctx, |ui| {
        ui.label("Open a reference, then select it here to keep it pinned.");
        if ui.button("Open reference image").clicked() {
            import = true;
        }
        egui::ComboBox::from_id_salt("reference-source")
            .selected_text(docs.iter().find(|d| Some(d.0) == pinned).map_or("Choose image", |d| d.1.as_str()))
            .show_ui(ui, |ui| {
                for (id, name, _) in &docs {
                    ui.selectable_value(&mut pinned, Some(*id), name);
                }
            });
        if let Some(index) = docs.iter().position(|d| Some(d.0) == pinned) {
            if let Some((texture, _)) = photocraft_ui_egui::canvas::ensure_texture(app, ctx, index, Some(512)) {
                let size = docs[index].2;
                let aspect = size.width as f32 / size.height.max(1) as f32;
                let height = 260f32.min(ui.available_width().max(100.) / aspect.max(0.001));
                let response = ui.add(egui::Image::new((texture, egui::vec2(height * aspect, height))));
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("reference-pick"), (response.rect, docs[index].0)));
            }
        }
        if let Some(doc) = app.session.active().map(|d| d.doc.clone()) {
            for (_, _, layer) in doc.walk().into_iter().filter(|(_, _, l)| l.reference) {
                ui.label(&layer.name);
                let tex = app.layer_thumb(ctx, &doc, layer);
                ui.add(egui::Image::new((tex, egui::vec2(128., 128.))));
            }
        }
    });
    if let Some(id) = pinned {
        ctx.data_mut(|d| d.insert_temp(key, id));
    }
    if !open {
        let mut s = settings.lock().unwrap_or_else(PoisonError::into_inner);
        s.show_reference = false;
        s.changed = true;
    }
    if import {
        if let Err(e) = photocraft_ui_egui::menus::invoke(app, ctx, "file.open", serde_json::json!({})) {
            app.ui.status = e;
        }
    }
}
