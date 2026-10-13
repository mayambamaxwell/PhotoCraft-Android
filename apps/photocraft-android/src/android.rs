use crate::shell::{Dialog, Shell};
use android_activity::input::{Axis, MotionAction, ToolType};
use photocraft_ui_egui::{PhotocraftApp, Services};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, PoisonError,
};
pub fn start(android: android_activity::AndroidApp) {
    if let Err(e) = rayon::ThreadPoolBuilder::new().num_threads(2).build_global() {
        log::warn!("Android render worker limit: {e}");
    }
    android_logger::init_once(android_logger::Config::default().with_tag("PhotoCraft").with_max_level(log::LevelFilter::Info));
    let Some(folder) = android.external_data_path().or_else(|| android.internal_data_path()) else {
        log::error!("No app data directory");
        return;
    };
    if let Err(e) = std::fs::create_dir_all(&folder) {
        log::error!("App storage: {e}");
        return;
    }
    crate::platform::init(folder.clone());
    let tablet = Arc::new(std::sync::Mutex::new(
        std::fs::File::open(folder.join("tablet.json"))
            .ok()
            .and_then(|f| crate::storage::read_bounded(f, 1 << 20).ok())
            .and_then(|b| serde_json::from_slice::<crate::tablet::Settings>(&b).ok())
            .unwrap_or_default(),
    ));
    {
        let mut settings = tablet.lock().unwrap_or_else(PoisonError::into_inner);
        settings.touch_size = if settings.touch_size.is_finite() { settings.touch_size.clamp(32., 52.) } else { 36. };
        settings.quick.truncate(12);
    }
    let tablet_preferences = tablet.clone();
    let dialogs: Dialog = Arc::default();
    let pending = dialogs.clone();
    let directory = folder.clone();
    let prefs = folder.join("preferences.json");
    let load = prefs.clone();
    let (workspace, mut recovery) = crate::workspace::Workspace::new(folder.clone());
    let pen_only = {
        let mut settings = tablet.lock().unwrap_or_else(PoisonError::into_inner);
        if !folder.join("tablet.json").exists() {
            settings.pen_only = workspace.pen_only();
        }
        settings.pen_only
    };
    let timelapse = crate::timelapse::Recorder::new(folder.clone());
    let timelapse_request = timelapse.requested.clone();
    let services = Services {
        open_url: Some(Box::new(crate::platform::open_url)),
        timelapse_export: Some(Box::new(move || timelapse_request.store(true, Ordering::Relaxed))),
        platform_preferences: Some(Box::new(move |ui| crate::tablet::preferences(ui, &tablet_preferences))),
        recover: recovery.recover.take(),
        adopt_autosave: recovery.adopt_autosave.take(),
        import: Some(Box::new(|name, bytes, depth| {
            photocraft_io::import_with_svg_group_depth(name, bytes, depth).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
        })),
        export: Some(Box::new(encode)),
        save_file: Some(Arc::new(|doc, path, settings, control| {
            control.check()?;
            control.progress(0.0, "Encoding");
            let (bytes, warnings) = encode(doc, path, settings)?;
            control.commit()?;
            control.progress(0.9, "Saving");
            crate::storage::write_atomic(path, &bytes)?;
            crate::platform::finish_export(path)?;
            Ok(warnings)
        })),
        write: Some(Box::new(|path, bytes| {
            crate::storage::write_atomic(path, bytes)?;
            crate::platform::finish_export(path)
        })),
        file_dialog: Some(Box::new(move |request, _, reply| {
            if let Err(e) = crate::platform::pick(&request) {
                log::warn!("Android picker: {e}");
            }
            *pending.lock().unwrap_or_else(PoisonError::into_inner) = Some((request, reply));
        })),
        load_prefs: Some(Box::new(move || {
            std::fs::File::open(&load).ok().and_then(|file| crate::storage::read_bounded(file, 1 << 20).ok()).and_then(|bytes| String::from_utf8(bytes).ok())
        })),
        save_prefs: Some(Box::new(move |text| crate::storage::write_atomic(&prefs.to_string_lossy(), text.as_bytes()))),
        ..Default::default()
    };
    let options = eframe::NativeOptions { android_app: Some(android), run_and_return: false, ..Default::default() };
    if let Err(error) = eframe::run_native(
        "PhotoCraft",
        options,
        Box::new(move |cc| {
            PhotocraftApp::setup_context(&cc.egui_ctx, photocraft_ui_egui::theme::ThemeKind::Pro);
            let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services);
            app.background_jobs = true;
            app.defer_workspace_recovery();
            if let Err(error)=app.run("prefs.set",serde_json::json!({"values":{"performance.memoryUsageMb":512,"performance.historyStates":24,"performance.effectCacheMb":64,"performance.cacheTileSize":512,"fileHandling.recoverOnLaunch":false}})){log::warn!("Android memory settings: {error}");}
            if let Err(error) = app.run("tools.setBrush", serde_json::json!({"preset":"Hard Round Pressure Size"})) {
                log::warn!("Android pressure brush: {error}");
            }
            if let Some(render) = cc.wgpu_render_state.clone() {
                app.set_wgpu(render);
            }
            let feed = app.stylus.feed.clone();
            let in_range = Arc::new(AtomicBool::new(false));
            let near = in_range.clone();
            let palm = Arc::new(AtomicBool::new(tablet.lock().unwrap_or_else(PoisonError::into_inner).palm));
            winit::platform::android::set_motion_filter(Arc::new(move |event| {
                for pointer in event.pointers() {
                    if matches!(pointer.tool_type(), ToolType::Stylus | ToolType::Eraser) {
                        let action = event.action();
                        near.store(!matches!(action, MotionAction::HoverExit | MotionAction::Cancel), Ordering::Relaxed);
                        if matches!(action, MotionAction::Down | MotionAction::PointerDown | MotionAction::Move) {
                            let (x, y) = crate::pen::tilt_degrees(pointer.axis_value(Axis::Tilt), pointer.orientation());
                            feed.set(Some(photocraft_ui_egui::stylus::PenSample {
                                pressure: pointer.pressure(),
                                tilt_x: x,
                                tilt_y: y,
                                rotation: 0.0,
                                eraser: pointer.tool_type() == ToolType::Eraser || event.button_state().stylus_primary(),
                            }));
                        } else if !matches!(action, MotionAction::Up | MotionAction::PointerUp) {
                            feed.set(None);
                        }
                    }
                }
                true
            }));
            Ok(Box::new(Shell {
                app,
                timelapse,
                tablet: tablet.clone(),
                dialogs,
                folder: directory,
                name: String::new(),
                error: String::new(),
                in_range,
                palm,
                pen_only,
                gestures: Default::default(),
                selection: Default::default(),
                workspace: Some(workspace),
            }))
        }),
    ) {
        log::error!("Android editor failed: {error}");
    }
}

fn encode(doc: &photocraft_doc::Document, path: &str, settings: &photocraft_ui_egui::ExportSettings) -> Result<(Vec<u8>, Vec<String>), String> {
    if path.to_ascii_lowercase().ends_with(".gif") && doc.timeline.is_some() {
        let frames = doc.timeline.as_ref().map_or(1, |t| t.duration).max(1);
        if (doc.size.width as u64).saturating_mul(doc.size.height as u64).saturating_mul(frames as u64) > 64_000_000 {
            return Err("Animation is too large for a safe GIF export. Reduce its size or split it into shorter scenes.".into());
        }
        let mut session = photocraft_engine::Session::new();
        session.add_document(doc.clone(), None);
        let mut bytes = Vec::new();
        photocraft_engine::render_video_with(
            &session,
            &serde_json::json!({"format":"gif"}),
            |_, data| {
                bytes.extend_from_slice(data);
                Ok(())
            },
            |_, _| true,
        )
        .map_err(|e| e.to_string())?;
        return Ok((bytes, Vec::new()));
    }
    let mut options = photocraft_io::ExportOptions::default();
    if let Some(q) = settings.jpeg_quality {
        options.encode.jpeg_quality = q;
    }
    options.encode.webp_lossless = settings.webp_lossless;
    if let Some(q) = settings.webp_quality {
        options.encode.webp_quality = q;
    }
    options.tiff_layers = settings.tiff_layers;
    options.xmp = if settings.xmp_all { photocraft_io::XmpEmbed::All } else { photocraft_io::XmpEmbed::None };
    options.tga_bits = settings.tga_bits;
    photocraft_io::export(doc, path, &options).map(|r| (r.bytes, r.warnings)).map_err(|e| e.to_string())
}
