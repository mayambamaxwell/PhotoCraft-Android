use egui_kittest::kittest::Queryable;
use photocraft_android::shell::Shell;
use photocraft_ui_egui::{state::DialogKind, PhotocraftApp, Services};
use std::sync::{atomic::AtomicBool, Arc};

#[test]
fn android_shell_initializes_editor_and_opens_new_document() {
    let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1691.0, 977.0)).with_max_steps(64).wgpu().build_eframe(|cc| make_shell(cc));
    harness.run_steps(4);
    harness.get_by_label("Create a new document or open an existing file.");
    if let Ok(path) = std::env::var("PHOTOCRAFT_STARTUP_SCREENSHOT") {
        harness.render().expect("render editor").save(path).expect("save editor screenshot");
    }
    let new_label = photocraft_ui_egui::shortcuts::command_label(&harness.state().app, "New document…", "file.new");
    let point = harness.get_by_label(&new_label).rect().center();
    native_step(&mut harness, vec![touch(0, egui::TouchPhase::Start, point)]);
    native_step(&mut harness, vec![touch(0, egui::TouchPhase::End, point)]);
    harness.run_steps(3);
    assert!(harness.state().app.ui.dialogs.iter().any(|dialog| matches!(dialog.kind, DialogKind::NewDocument)));
}

fn make_shell(cc: &eframe::CreationContext<'_>) -> Shell {
    PhotocraftApp::setup_context(&cc.egui_ctx, photocraft_ui_egui::theme::ThemeKind::Pro);
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Services::default());
    app.tablet_mode = true;
    if let Some(render) = cc.wgpu_render_state.clone() {
        app.set_wgpu(render);
    }
    Shell {
        app,
        tablet: Arc::default(),
        dialogs: Arc::default(),
        folder: std::env::temp_dir(),
        name: String::new(),
        error: String::new(),
        in_range: Arc::new(AtomicBool::new(false)),
        palm: Arc::new(AtomicBool::new(true)),
        pen_only: true,
        gestures: Default::default(),
        selection: Default::default(),
        timelapse: photocraft_android::timelapse::Recorder::new(std::env::temp_dir().join("photocraft-ui-recording-test")),
        workspace: None,
    }
}

// Reproduce preview.1's incomplete forwarding without changing production code.
struct WithoutLogic(Shell);
impl eframe::App for WithoutLogic {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        eframe::App::ui(&mut self.0, ui, frame);
    }
}
#[test]
fn missing_logic_reproduces_toolbar_only_screen() {
    let mut harness =
        egui_kittest::Harness::builder().with_size(egui::vec2(1691.0, 977.0)).with_max_steps(64).wgpu().build_eframe(|cc| WithoutLogic(make_shell(cc)));
    harness.run_steps(4);
    assert!(harness.query_by_label("PhotoCraft · Android preview.4").is_none());
    assert!(harness.query_by_label("Create a new document or open an existing file.").is_none());
}
fn touch(id: u64, phase: egui::TouchPhase, pos: egui::Pos2) -> egui::Event {
    egui::Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId((1 << 16) | id), phase, pos, force: None }
}
fn native_step(h: &mut egui_kittest::Harness<'static, Shell>, events: Vec<egui::Event>) {
    let ctx = h.ctx.clone();
    let mut raw = egui::RawInput { events, focused: true, ..Default::default() };
    eframe::App::raw_input_hook(h.state_mut(), &ctx, &mut raw);
    for event in raw.events {
        h.event(event);
    }
    h.step();
}
#[test]
fn two_fingers_transform_camera_without_editing_document() {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1691., 977.)).with_max_steps(64).wgpu().build_eframe(|cc| make_shell(cc));
    h.run_steps(4);
    h.state_mut().app.run("file.new", serde_json::json!({"width":256,"height":256,"background":"white"})).unwrap();
    h.run_steps(4);
    let xf = photocraft_ui_egui::canvas::ViewXform::active(&h.state().app).unwrap();
    let mid = xf.rect.center();
    let old = h.state().app.ui.views[0].clone();
    let revision = h.state().app.session.active().unwrap().revision;
    native_step(&mut h, vec![touch(0, egui::TouchPhase::Start, mid - egui::vec2(30., 0.))]);
    native_step(&mut h, vec![touch(1, egui::TouchPhase::Start, mid + egui::vec2(30., 0.))]);
    let d = egui::vec2(40., 30.);
    let offset = egui::vec2(40., 40.);
    native_step(&mut h, vec![touch(0, egui::TouchPhase::Move, mid + d - offset), touch(1, egui::TouchPhase::Move, mid + d + offset)]);
    let view = &h.state().app.ui.views[0];
    assert!(view.zoom > old.zoom);
    assert!((view.rotation - 45.).abs() < 0.01);
    assert_ne!(view.center, old.center);
    native_step(&mut h, vec![touch(1, egui::TouchPhase::End, mid + d + offset)]);
    native_step(&mut h, vec![touch(0, egui::TouchPhase::Move, mid)]);
    native_step(&mut h, vec![touch(0, egui::TouchPhase::End, mid)]);
    assert_eq!(h.state().app.session.active().unwrap().revision, revision, "gesture and remaining finger must never paint");
    native_step(&mut h, vec![touch(0, egui::TouchPhase::Start, mid)]);
    native_step(&mut h, vec![touch(0, egui::TouchPhase::Move, mid + egui::vec2(20., 0.))]);
    native_step(&mut h, vec![touch(0, egui::TouchPhase::End, mid + egui::vec2(20., 0.))]);
    assert_eq!(h.state().app.session.active().unwrap().revision, revision, "pen-only blocks finger brush marks");
    h.state_mut().pen_only = false;
    native_step(&mut h, vec![touch(0, egui::TouchPhase::Start, mid)]);
    native_step(&mut h, vec![touch(0, egui::TouchPhase::End, mid)]);
    assert!(h.state().app.session.active().unwrap().revision > revision, "finger drawing option must work");
    h.state().in_range.store(true, std::sync::atomic::Ordering::Relaxed);
    let ctx = h.ctx.clone();
    let mut raw = egui::RawInput { focused: false, events: vec![egui::Event::WindowFocused(false)], ..Default::default() };
    eframe::App::raw_input_hook(h.state_mut(), &ctx, &mut raw);
    assert!(!h.state().in_range.load(std::sync::atomic::Ordering::Relaxed), "backgrounding clears stale pen proximity");
}

#[test]
fn selection_bar_commands_and_multifinger_history() {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1691., 977.)).with_max_steps(64).wgpu().build_eframe(|cc| make_shell(cc));
    h.run_steps(4);
    h.state_mut().app.run("file.new", serde_json::json!({"width":64,"height":64,"background":"transparent"})).unwrap();
    h.state_mut().app.run("edit.fill", serde_json::json!({"color":"#ff0000"})).unwrap();
    h.state_mut().app.run("select.rect", serde_json::json!({"x":16,"y":16,"width":16,"height":16})).unwrap();
    h.run_steps(4);
    for label in [
        "Deselect",
        "Invert selection",
        "Expand",
        "Shrink",
        "Delete selected area",
        "Cut to new layer",
        "Copy to new layer",
        "Scale / rotate",
        "Free form",
        "Feather",
        "Transform selection",
    ] {
        h.get_by_label(label);
    }
    if let Ok(path) = std::env::var("PHOTOCRAFT_SELECTION_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
    h.get_by_label("Expand").click();
    h.run_steps(3);
    let bounds = h.state().app.session.active().unwrap().doc.selection.as_ref().unwrap().content_bounds();
    assert_eq!(bounds.width(), 18);
    h.get_by_label("Shrink").click();
    h.run_steps(3);
    assert_eq!(h.state().app.session.active().unwrap().doc.selection.as_ref().unwrap().content_bounds().width(), 16);
    h.get_by_label("Invert selection").click();
    h.run_steps(3);
    assert_eq!(h.state().app.session.active().unwrap().doc.selection.as_ref().unwrap().content_bounds().width(), 64);
    h.get_by_label("Invert selection").click();
    h.run_steps(3);
    h.get_by_label("Copy to new layer").click();
    h.run_steps(3);
    assert_eq!(h.state().app.session.active().unwrap().doc.layers.len(), 2);
    assert!(h.state().app.session.active().unwrap().doc.selection.is_none());
    let mid = photocraft_ui_egui::canvas::ViewXform::active(&h.state().app).unwrap().rect.left_top() + egui::vec2(150., 150.);
    h.state_mut().pen_only = false;
    tap_group(&mut h, mid, 2);
    assert_eq!(h.state().app.session.active().unwrap().doc.layers.len(), 1, "two-finger tap undoes copy without painting");
    tap_group(&mut h, mid, 3);
    assert_eq!(h.state().app.session.active().unwrap().doc.layers.len(), 2, "three-finger tap redoes copy");
    h.state_mut().app.run("select.rect", serde_json::json!({"x":16,"y":16,"width":16,"height":16})).unwrap();
    h.run_steps(3);
    h.get_by_label("Scale / rotate").click();
    h.run_steps(3);
    assert!(h.state().app.ui.transform.is_some());
    h.get_by_label("Cancel selection transform").click();
    h.run_steps(3);
    assert!(h.state().app.ui.transform.is_none());
    h.get_by_label("Free form").click();
    h.run_steps(3);
    assert!(h.state().app.ui.transform.is_some());
    h.get_by_label("Cancel selection transform").click();
    h.run_steps(3);
    h.get_by_label("Cut to new layer").click();
    h.run_steps(3);
    assert_eq!(h.state().app.session.active().unwrap().doc.layers.len(), 3);
    let d = h.state().app.session.active().unwrap();
    let source = d.doc.layers[1].surface().unwrap();
    let mut pixels = vec![[0.; 4]; 4096];
    source.read_rgba_into(d.doc.bounds(), &mut pixels);
    assert_eq!(pixels[16 * 64 + 16][3], 0., "cut removes source pixels");
    h.state_mut().app.run("select.rect", serde_json::json!({"x":16,"y":16,"width":16,"height":16})).unwrap();
    h.run_steps(3);
    h.get_by_label("Delete selected area").click();
    h.run_steps(3);
    let d = h.state().app.session.active().unwrap();
    let surface = d.doc.layer(d.active_layer.unwrap()).unwrap().surface().unwrap();
    surface.read_rgba_into(d.doc.bounds(), &mut pixels);
    assert_eq!(pixels[16 * 64 + 16][3], 0., "delete clears selected pixels");
    h.get_by_label("Deselect").click();
    h.run_steps(3);
    assert!(h.state().app.session.active().unwrap().doc.selection.is_none());
}
fn tap_group(h: &mut egui_kittest::Harness<'static, Shell>, mid: egui::Pos2, count: u64) {
    native_step(h, (0..count).map(|id| touch(id, egui::TouchPhase::Start, mid + egui::vec2(id as f32 * 30., 0.))).collect());
    native_step(h, (0..count).map(|id| touch(id, egui::TouchPhase::End, mid + egui::vec2(id as f32 * 30., 0.))).collect());
}
#[test]
fn tablet_layer_controls_and_drawing_timeline() {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1691., 977.)).with_max_steps(64).wgpu().build_eframe(|cc| make_shell(cc));
    h.run_steps(4);
    h.state_mut().app.run("file.new", serde_json::json!({"width":256,"height":256,"background":"transparent"})).unwrap();
    h.run_steps(4);
    for label in ["Clip to layer below", "Lock transparent pixels", "Set as reference layer", "Import image", "Hue, saturation and lightness"] {
        h.get_by_label(label);
    }
    let lock = h.get_by_label("Lock transparent pixels").rect();
    let new = h.get_by_label("Create a new layer").rect();
    assert!(new.top() >= lock.bottom());
    assert!(h.query_by_label("Discord").is_none());
    let ctx = h.ctx.clone();
    photocraft_ui_egui::menus::invoke(&mut h.state_mut().app, &ctx, "window.panel.timeline", serde_json::json!({})).unwrap();
    h.run_steps(4);
    let pos = h.get_by_label("Create drawing animation").rect().center();
    native_step(&mut h, vec![touch(0, egui::TouchPhase::Start, pos)]);
    native_step(&mut h, vec![touch(0, egui::TouchPhase::End, pos)]);
    h.run_steps(3);
    let d = h.state().app.session.active().unwrap();
    assert_eq!(d.doc.timeline.as_ref().unwrap().fps, 12.);
    assert!(d.doc.layer(d.active_layer.unwrap()).unwrap().video.is_some());
    h.get_by_label("Blank frame").click();
    h.run_steps(3);
    assert_eq!(h.state().app.session.active().unwrap().doc.timeline.as_ref().unwrap().current, 1);
    h.get_by_label("Onion skin").click();
    h.run_steps(3);
    assert!(h.state().app.ui.timeline.onion);
    if let Ok(path) = std::env::var("PHOTOCRAFT_TABLET_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
}
#[test]
fn recovery_waits_for_explicit_restore() {
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = calls.clone();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1691., 977.)).with_max_steps(64).wgpu().build_eframe(move |cc| {
        let mut shell = make_shell(cc);
        shell.app.services.recover = Some(Box::new(move || {
            seen.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Vec::new()
        }));
        shell.app.defer_workspace_recovery();
        shell
    });
    h.run_steps(4);
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 0);
    h.state_mut().app.recover_workspace().unwrap();
    h.run_steps(4);
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
}
#[test]
fn feather_and_outline_transform_preserve_layer_pixels() {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1691., 977.)).with_max_steps(64).wgpu().build_eframe(|cc| make_shell(cc));
    h.run_steps(4);
    h.state_mut().app.run("file.new", serde_json::json!({"width":64,"height":64,"background":"#00ff00"})).unwrap();
    h.state_mut().app.run("select.rect", serde_json::json!({"x":16,"y":16,"width":16,"height":16})).unwrap();
    h.run_steps(4);
    let pixels = photocraft_io::export(&h.state().app.session.active().unwrap().doc, "test.png", &Default::default()).unwrap().bytes;
    h.get_by_label("Feather").click();
    h.run_steps(3);
    assert!(h.state().app.session.active().unwrap().doc.selection.as_ref().unwrap().content_bounds().width() > 16);
    h.state_mut().app.run("edit.undo", serde_json::json!({})).unwrap();
    h.run_steps(3);
    h.get_by_label("Transform selection").click();
    h.run_steps(3);
    let t = h.state_mut().app.ui.transform.as_mut().unwrap();
    assert!(t.selection);
    for p in &mut t.quad {
        p[0] += 4.;
    }
    photocraft_ui_egui::transform_tool::commit(&mut h.state_mut().app);
    h.run_steps(3);
    let doc = &h.state().app.session.active().unwrap().doc;
    assert_eq!(doc.selection.as_ref().unwrap().content_bounds().x0, 20);
    assert_eq!(photocraft_io::export(doc, "test.png", &Default::default()).unwrap().bytes, pixels);
}

#[test]
fn portrait_welcome_credit_and_timelapse_export_dialog() {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(800., 1280.)).with_max_steps(64).wgpu().build_eframe(|cc| make_shell(cc));
    h.run_steps(4);
    h.get_by_label("Ported to Android by mayamba");
    h.state_mut().app.run("file.new", serde_json::json!({"width":64,"height":96,"background":"white"})).unwrap();
    let flag = h.state().timelapse.requested.clone();
    h.state_mut().app.services.timelapse_export = Some(Box::new(move || flag.store(true, std::sync::atomic::Ordering::Relaxed)));
    let ctx = h.ctx.clone();
    photocraft_ui_egui::menus::invoke(&mut h.state_mut().app, &ctx, "file.export.timelapse", serde_json::json!({})).unwrap();
    h.run_steps(4);
    for label in ["10s", "30s", "60s", "120s", "300s"] {
        h.get_by_label(label);
    }
    h.get_by_label("Generate and save video to Gallery");
    if let Ok(path) = std::env::var("PHOTOCRAFT_PORTRAIT_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
}
