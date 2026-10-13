//! Native Android host; no browser or WebView.
#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "android", test))]
mod pen;
mod storage;
// Platform-required export attribute; no unsafe blocks or raw pointer operations.
#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    android::start(app);
}

pub mod shell;

pub mod gestures;
pub mod workspace;

pub mod selection;

#[cfg(target_os = "android")]
mod platform;
pub mod reference;
pub mod tablet;

pub mod timelapse;

#[cfg(test)]
mod file_tests;
