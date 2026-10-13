//! Bounded, cached onion skins for the active drawing layer.
use crate::PhotocraftApp;
pub fn paint(app: &PhotocraftApp, ctx: &egui::Context) {
    if !app.ui.timeline.onion || app.ui.timeline.playing {
        return;
    }
    let Some(st) = app.session.active() else {
        return;
    };
    let Some(t) = &st.doc.timeline else {
        return;
    };
    let Some(video) = st.active_layer.and_then(|id| st.doc.layer(id)).and_then(|l| l.video.as_ref()) else {
        return;
    };
    let Some(xf) = crate::canvas::ViewXform::active(app) else {
        return;
    };
    let (wd, hd) = (st.doc.size.width, st.doc.size.height);
    if wd == 0 || hd == 0 {
        return;
    }
    let scale = (512. / wd.max(hd) as f32).min(1.);
    let w = (wd as f32 * scale).max(1.) as usize;
    let h = (hd as f32 * scale).max(1.) as usize;
    for (slot, (frame, tint)) in [(t.current.checked_sub(1), [255, 70, 70]), (t.current.checked_add(1), [70, 160, 255])].into_iter().enumerate() {
        let Some(index) = frame else {
            continue;
        };
        let Some(surface) = video.frames.get(index) else {
            continue;
        };
        let id = egui::Id::new(("onion-image", slot));
        let fingerprint = surface.tiles().fold(0usize, |v, (_, t)| v.wrapping_add(std::sync::Arc::as_ptr(t) as usize));
        let cache_key = (st.doc.id.0, st.active_layer, index, fingerprint);
        let cached = ctx.data(|d| d.get_temp::<((u64, Option<photocraft_doc::LayerId>, usize, usize), egui::TextureHandle)>(id));
        let texture = if let Some((_, texture)) = cached.filter(|c| c.0 == cache_key) {
            texture
        } else {
            let mut image = egui::ColorImage::filled([w, h], egui::Color32::TRANSPARENT);
            let mut pixel = [0.; 8];
            let n = surface.channels().min(8);
            for (i, out) in image.pixels.iter_mut().enumerate() {
                surface.read_pixel(((i % w) as f32 / scale) as i32, ((i / w) as f32 / scale) as i32, &mut pixel[..n]);
                let a = photocraft_raster::to_rgba(&surface.format(), &pixel[..n])[3];
                *out = egui::Color32::from_rgba_unmultiplied(tint[0], tint[1], tint[2], (a.clamp(0., 1.) * 65.) as u8);
            }
            let texture = ctx.load_texture(format!("onion-{index}"), image, egui::TextureOptions::LINEAR);
            ctx.data_mut(|d| d.insert_temp(id, (cache_key, texture.clone())));
            texture
        };
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("onion-skins"))).with_clip_rect(xf.rect);
        let mut mesh = egui::Mesh::with_texture(texture.id());
        for ((x, y), (u, v)) in [((0., 0.), (0., 0.)), ((wd as f32, 0.), (1., 0.)), ((wd as f32, hd as f32), (1., 1.)), ((0., hd as f32), (0., 1.))] {
            mesh.vertices.push(egui::epaint::Vertex { pos: xf.to_screen(x, y), uv: egui::pos2(u, v), color: egui::Color32::WHITE });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        painter.add(egui::Shape::mesh(mesh));
    }
}
