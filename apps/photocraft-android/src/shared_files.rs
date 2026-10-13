//! Optional direct file access through Android's user-granted storage settings.
use jni::objects::JString;
jni::bind_java_type! { Environment => "android.os.Environment", methods { static fn is_external_storage_manager() -> bool, } }
jni::bind_java_type! { Uri => "android.net.Uri", methods { static fn parse(text:JString) -> Uri, } }
jni::bind_java_type! { Intent => "android.content.Intent", type_map { Uri => "android.net.Uri", }, constructors { fn new(action:JString), }, methods { fn set_data(data:Uri) -> Intent, fn add_flags(flags:i32) -> Intent, } }
jni::bind_java_type! { Context => "android.content.Context", type_map { Intent => "android.content.Intent", }, methods { fn start_activity(intent:Intent), } }
pub fn open_shared_files() -> Result<bool, String> {
    jni_min_helper::jni_with_env(|env| {
        if Environment::is_external_storage_manager(env)? {
            return Ok(true);
        }
        let action = JString::new(env, "android.settings.MANAGE_APP_ALL_FILES_ACCESS_PERMISSION")?;
        let intent = Intent::new(env, &action)?;
        let package = JString::new(env, "package:com.mayamba.photocraftpreview")?;
        let uri = Uri::parse(env, &package)?;
        intent.set_data(env, &uri)?;
        intent.add_flags(env, 0x10000000)?;
        let context = env.as_cast::<Context>(jni_min_helper::android_context())?;
        context.start_activity(env, &intent)?;
        Ok(false)
    })
    .map_err(|e| e.to_string())
}
