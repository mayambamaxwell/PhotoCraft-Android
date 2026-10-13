use photocraft_engine::Session;
use photocraft_geom::Rect;
use serde_json::json;
const EFFECTS: [&str; 5] = [
    "filter.artistic.bloom",
    "filter.artistic.crossFilter",
    "filter.artistic.noise",
    "filter.artistic.chromaticAberrationZooming",
    "filter.artistic.chromaticAberrationMoving",
];
fn session(depth: u32, mode: &str) -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width":32,"height":32,"background":"transparent","depth":depth,"mode":mode})).unwrap();
    s.edit("bright spot", |doc, id| {
        let fmt = doc.pixel_format();
        let surface = doc.layer_mut(id.unwrap()).unwrap().surface_mut().unwrap();
        let px = photocraft_raster::from_rgba(&fmt, [1., 0.8, 0.9, 1.]);
        surface.write_region(Rect::new(12, 12, 20, 20), &px.repeat(64));
        Ok(())
    })
    .unwrap();
    s
}
fn sample(s: &Session, x: i32, y: i32) -> [f32; 4] {
    let d = s.active().unwrap();
    let surface = d.doc.layer(d.active_layer.unwrap()).unwrap().surface().unwrap();
    let mut p = [[0.; 4]];
    surface.read_rgba_into(Rect::new(x, y, x + 1, y + 1), &mut p);
    p[0]
}
#[test]
fn effects_respect_selection_depth_and_undo() {
    for (depth, mode) in [(8, "rgb"), (16, "rgb"), (32, "rgb"), (8, "cmyk"), (16, "lab"), (8, "gray")] {
        for id in EFFECTS {
            let mut s = session(depth, mode);
            s.execute("select.rect", json!({"x":10,"y":10,"width":10,"height":10})).unwrap();
            let before = sample(&s, 16, 16);
            let outside = sample(&s, 23, 16);
            s.execute(id, json!({})).unwrap();
            assert_eq!(sample(&s, 23, 16), outside, "{id} outside selection");
            assert!(sample(&s, 16, 16).iter().all(|v| v.is_finite()));
            s.execute("edit.undo", json!({})).unwrap();
            assert_eq!(sample(&s, 16, 16), before, "{id} undo");
        }
    }
}
#[test]
fn bloom_spreads_and_zero_chromatic_shift_is_identity() {
    let mut s = session(32, "rgb");
    assert_eq!(sample(&s, 10, 16)[3], 0.);
    s.execute(EFFECTS[0], json!({"radius":3.,"threshold":0.5})).unwrap();
    assert!(sample(&s, 10, 16)[3] > 0.);
    let before = sample(&s, 16, 16);
    s.execute(EFFECTS[4], json!({"distance":0})).unwrap();
    assert_eq!(sample(&s, 16, 16), before);
}
#[test]
fn invalid_effects_leave_document_unchanged() {
    for id in EFFECTS {
        for p in [json!({"unknown":true}), json!({"radius":-1}), json!({"distance":"bad"})] {
            let mut s = session(8, "rgb");
            let revision = s.active().unwrap().revision;
            assert!(s.execute(id, p).is_err(), "{id}");
            assert_eq!(s.active().unwrap().revision, revision);
        }
    }
}
#[test]
fn drawing_frames_and_reference_layer_roundtrip() {
    let mut s = session(8, "rgb");
    s.execute("timeline.create", json!({"duration":1,"fps":12})).unwrap();
    s.execute("layer.videoLayers.newBlankVideoLayer", json!({})).unwrap();
    s.execute("layer.setProps", json!({"reference":true})).unwrap();
    s.execute("edit.fill", json!({"color":"#ff0000"})).unwrap();
    s.execute("layer.videoLayers.insertBlankFrame", json!({})).unwrap();
    s.execute("timeline.setFrame", json!({"frame":1})).unwrap();
    s.execute("edit.fill", json!({"color":"#0000ff"})).unwrap();
    let doc = &s.active().unwrap().doc;
    let bytes = photocraft_format::save_to_bytes(doc, &Default::default()).unwrap();
    let doc = photocraft_format::load_from_bytes(&bytes).unwrap();
    let layer = doc.layer(s.active().unwrap().active_layer.unwrap()).unwrap();
    assert!(layer.reference);
    let video = layer.video.as_ref().unwrap();
    assert_eq!(video.frames.len(), 2);
    for (frame, channel) in [(0, 0), (1, 2)] {
        let mut pixel = [[0.; 4]];
        video.frames[frame].read_rgba_into(Rect::new(0, 0, 1, 1), &mut pixel);
        assert!(pixel[0][channel] > 0.99);
    }
    s.execute("layer.videoLayers.deleteFrame", json!({})).unwrap();
    assert_eq!(s.active().unwrap().doc.timeline.as_ref().unwrap().duration, 1);
}
#[test]
fn finishing_effects_change_edges_and_noise_is_repeatable() {
    let mut moving = session(32, "rgb");
    let old = sample(&moving, 12, 16);
    moving.execute(EFFECTS[4], json!({"distance":3,"angle":0})).unwrap();
    let shifted = sample(&moving, 12, 16);
    assert!(shifted[0] < 0.1 && shifted[2] > 0.8);
    assert_eq!(shifted[3], old[3]);
    let mut zoom = session(32, "rgb");
    zoom.edit("opaque test background", |doc, id| {
        let fmt = doc.pixel_format();
        let surface = doc.layer_mut(id.unwrap()).unwrap().surface_mut().unwrap();
        surface.write_region(Rect::new(0, 0, 32, 32), &photocraft_raster::from_rgba(&fmt, [0., 0., 0., 1.]).repeat(1024));
        surface.write_region(Rect::new(12, 12, 20, 20), &photocraft_raster::from_rgba(&fmt, [1., 0.8, 0.9, 1.]).repeat(64));
        Ok(())
    })
    .unwrap();
    zoom.execute(EFFECTS[3], json!({"distance":20})).unwrap();
    assert!((sample(&zoom, 12, 16)[2] - old[2]).abs() > 0.1);
    let mut cross = session(32, "rgb");
    cross.execute(EFFECTS[1], json!({"radius":4,"threshold":0.5,"count":2,"direction":0})).unwrap();
    assert!(sample(&cross, 10, 16)[3] > 0.);
    let mut noise = session(16, "rgb");
    let old = sample(&noise, 16, 16);
    noise.execute(EFFECTS[2], json!({"amount":20,"seed":29})).unwrap();
    let changed = sample(&noise, 16, 16);
    assert_ne!(changed, old);
    noise.execute("edit.undo", json!({})).unwrap();
    noise.execute(EFFECTS[2], json!({"amount":20,"seed":29})).unwrap();
    assert_eq!(sample(&noise, 16, 16), changed);
}
