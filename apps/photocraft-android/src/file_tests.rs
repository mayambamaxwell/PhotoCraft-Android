use photocraft_ui_egui::{PhotocraftApp, Services};
#[test]
fn project_save_reopens_pixels_and_records_a_real_recent_path() {
    let folder = std::env::temp_dir().join(format!("photocraft-files-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut app = PhotocraftApp::new(
        photocraft_engine::Session::new(),
        Services {
            import: Some(Box::new(|name, bytes, depth| {
                photocraft_io::import_with_svg_group_depth(name, bytes, depth).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
            })),
            ..Default::default()
        },
    );
    app.run("file.new", serde_json::json!({"width":16,"height":16,"background":"white"})).unwrap();
    app.run("edit.fill", serde_json::json!({"color":"#12ab34"})).unwrap();
    app.run("layer.duplicate", serde_json::json!({})).unwrap();
    let doc = app.session.active().unwrap().doc.clone();
    let path = folder.join("Saved.pcraft");
    let encoded = photocraft_io::export(&doc, "Saved.pcraft", &Default::default()).unwrap().bytes;
    crate::storage::write_atomic(&path.to_string_lossy(), &encoded).unwrap();
    app.open_path(&path.to_string_lossy()).unwrap();
    assert_eq!(app.ui.recent_files.first().unwrap(), &path.to_string_lossy());
    assert_eq!(app.session.active().unwrap().path.as_deref(), path.to_str());
    assert!(path.exists());
    let reopened = app.session.active().unwrap().doc.clone();
    assert_eq!(reopened.layers.len(), 2);
    let before = photocraft_io::export(&doc, "pixels.png", &Default::default()).unwrap().bytes;
    let after = photocraft_io::export(&reopened, "pixels.png", &Default::default()).unwrap().bytes;
    assert_eq!(before, after);
    let prefs = serde_json::to_string(app.session.prefs()).unwrap();
    let mut session = photocraft_engine::Session::new();
    session.load_prefs_json(&prefs).unwrap();
    assert_eq!(session.prefs().file_handling.recent_files.first().unwrap(), &path.to_string_lossy());
    std::fs::remove_dir_all(folder).unwrap();
}
