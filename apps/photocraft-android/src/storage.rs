//! Bounded reads and atomic saves for the preview's app-owned folder.
use std::io::{Read, Write};
pub fn read_bounded(reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader.take(limit.saturating_add(1)).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("File exceeds the {limit}-byte Android safety limit. Use a smaller file."));
    }
    Ok(bytes)
}
pub fn write_atomic(path: &str, bytes: &[u8]) -> Result<(), String> {
    let path = std::path::Path::new(path);
    static NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut name = path.file_name().ok_or("Missing file name")?.to_os_string();
    let nonce = NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    name.push(format!(".save-tmp-{}-{stamp}-{nonce}", std::process::id()));
    let temporary = path.with_file_name(name);
    let mut created = false;
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        created = true;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })()
    .map_err(|e: std::io::Error| format!("Cannot save {}: {e}. Check free storage and folder permissions.", path.display()));
    if result.is_err() && created {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
#[cfg(test)]
mod tests {
    #[test]
    fn rejects_oversized_stream_and_preserves_exact_bytes() {
        assert!(super::read_bounded(&b"123456789"[..], 8).is_err());
        assert_eq!(super::read_bounded(&b"12345678"[..], 8).unwrap(), b"12345678");
    }
}

#[cfg(test)]
mod atomic_tests {
    #[test]
    fn replaces_only_after_complete_write() {
        let path = std::env::temp_dir().join(format!("photocraft-atomic-{}", std::process::id()));
        std::fs::write(&path, b"old").unwrap();
        super::write_atomic(path.to_str().unwrap(), b"new pixels").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new pixels");
        assert!(!path.with_extension("photocraft-save-tmp").exists());
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod failure_tests {
    #[test]
    fn failed_replace_preserves_destination_and_removes_temporary_file() {
        let folder = std::env::temp_dir().join(format!("photocraft-failed-save-{}", std::process::id()));
        let destination = folder.join("project.pcraft");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("keep"), b"existing data").unwrap();
        let error = super::write_atomic(destination.to_str().unwrap(), b"new").unwrap_err();
        assert!(error.contains("Check free storage"));
        assert_eq!(std::fs::read(destination.join("keep")).unwrap(), b"existing data");
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
        std::fs::remove_dir_all(folder).unwrap();
    }
}
