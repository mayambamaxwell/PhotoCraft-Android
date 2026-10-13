//! Narrow Android document-picker and share bridge; editor rendering remains native Rust.
use jni::objects::JString;
use photocraft_ui_egui::{FileDialogAnswer, FileDialogReply, FileDialogRequest, PhotocraftApp};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock, PoisonError,
    },
};
static FILES: OnceLock<PathBuf> = OnceLock::new();
static PICKER: AtomicBool = AtomicBool::new(false);
static ERROR: Mutex<Option<String>> = Mutex::new(None);
jni::bind_java_type! { Context=>"android.content.Context", methods { fn get_class_loader()->JClassLoader, } }
jni::bind_java_type! { Activity=>"com.mayamba.photocraftpreview.PhotoCraftActivity",
 hooks { load_class = |env, _context, _initialize| {
  let context = env.as_cast::<Context>(jni_min_helper::android_context())?;
  let loader = context.get_class_loader(env)?;
  let name = JString::new(env, "com.mayamba.photocraftpreview.PhotoCraftActivity")?;
  loader.load_class(env, &name)
 } },
 methods { static fn current()->Activity, fn open_external(url:JString), fn pick_document(save:bool,name:JString), fn finish_export(path:JString)->JString, fn take_picker_result()->JString, fn take_platform_error()->JString, fn export_timelapse(folder:JString,seconds:i32)->JString, }
}

pub fn init(path: PathBuf) {
    let _ = FILES.set(path);
}
pub fn picker_active() -> bool {
    PICKER.load(Ordering::Relaxed)
}
pub fn pick(request: &FileDialogRequest) -> Result<(), String> {
    FILES.get().ok_or("Android file storage unavailable")?;
    let (save, name) = match request {
        FileDialogRequest::Save { suggested } => (true, std::path::Path::new(suggested).file_name().and_then(|n| n.to_str()).unwrap_or("Untitled.png")),
        _ => (false, "Image"),
    };
    jni_min_helper::jni_with_env(|env| {
        let activity = Activity::current(env)?;
        let name = JString::new(env, name)?;
        activity.pick_document(env, save, &name)
    })
    .map_err(|e| e.to_string())?;
    PICKER.store(true, Ordering::Relaxed);
    Ok(())
}
pub fn poll_picker() -> Result<Option<Option<FileDialogAnswer>>, String> {
    let result = jni_min_helper::jni_with_env(|env| {
        let activity = Activity::current(env)?;
        activity.take_picker_result(env)?.try_to_string(env)
    })
    .map_err(|e| e.to_string())?;
    if result.is_empty() {
        return Ok(None);
    }
    PICKER.store(false, Ordering::Relaxed);
    if result.len() > 65536 {
        return Err("Android file result is too large".into());
    }
    let v: serde_json::Value = serde_json::from_str(&result).map_err(|e| e.to_string())?;
    match v["state"].as_str() {
        Some("cancel") => Ok(Some(None)),
        Some("error") => Err(v["error"].as_str().unwrap_or("Android file access failed").into()),
        Some("save") => Ok(Some(Some(FileDialogAnswer::SaveTo(v["path"].as_str().ok_or("Missing save path")?.into())))),
        Some("open") => Ok(Some(Some(FileDialogAnswer::Paths(vec![v["path"].as_str().ok_or("Missing import path")?.into()])))),
        _ => Err("Unexpected Android file picker response".into()),
    }
}
pub fn send_reply(reply: FileDialogReply, answer: Option<FileDialogAnswer>) {
    reply.send(answer);
}

pub fn take_error() -> Option<String> {
    ERROR.lock().unwrap_or_else(PoisonError::into_inner).take().or_else(|| {
        jni_min_helper::jni_with_env(|env| {
            let activity = Activity::current(env)?;
            activity.take_platform_error(env)?.try_to_string(env)
        })
        .ok()
        .filter(|s| !s.is_empty())
    })
}
pub fn finish_export(path: &str) -> Result<(), String> {
    let message = jni_min_helper::jni_with_env(|env| {
        let activity = Activity::current(env)?;
        let path = JString::new(env, path)?;
        activity.finish_export(env, &path)?.try_to_string(env)
    })
    .map_err(|e| e.to_string())?;
    if message.is_empty() {
        Ok(())
    } else {
        Err(message)
    }
}
pub fn share(app: &mut PhotocraftApp) -> Result<(), String> {
    let root = FILES.get().ok_or("Android file storage unavailable")?.join("exports");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(root.join("PhotoCraft-share.png.uri"));
    app.export_image_copy(root.join("PhotoCraft-share.png").to_string_lossy().into_owned()).map(|_| ())
}

pub fn timelapse(folder: &str, seconds: i32) -> Result<(), String> {
    let message = jni_min_helper::jni_with_env(|env| {
        let activity = Activity::current(env)?;
        let folder = JString::new(env, folder)?;
        activity.export_timelapse(env, &folder, seconds)?.try_to_string(env)
    })
    .map_err(|e| e.to_string())?;
    if message.is_empty() {
        Ok(())
    } else {
        Err(message)
    }
}

pub fn open_url(url: &str) -> Result<(), String> {
    jni_min_helper::jni_with_env(|env| {
        let activity = Activity::current(env)?;
        let url = JString::new(env, url)?;
        activity.open_external(env, &url)
    })
    .map_err(|e| e.to_string())
}
