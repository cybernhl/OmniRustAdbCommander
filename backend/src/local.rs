use async_trait::async_trait;
use crate::traits::DeviceBackend;
use adb_explorer_common::models::FileEntry;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub struct LocalBackend;

impl LocalBackend {
    pub fn new() -> Self {
        Self
    }

    #[cfg(windows)]
    pub fn get_logical_drives() -> Vec<(String, String)> {
        use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;
        use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let mut drives = Vec::new();
        let mask = unsafe { GetLogicalDrives() };
        for i in 0..26 {
            if (mask & (1 << i)) != 0 {
                let drive_root = format!("{}:\\", (b'A' + i as u8) as char);

                let mut label = [0u16; 256];
                let root_wide = OsStr::new(&drive_root).encode_wide().chain(Some(0)).collect::<Vec<_>>();

                let success = unsafe {
                    GetVolumeInformationW(
                        root_wide.as_ptr(),
                        label.as_mut_ptr(),
                        label.len() as u32,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0
                    )
                };

                let label_str = if success != 0 {
                    let len = label.iter().position(|&x| x == 0).unwrap_or(label.len());
                    String::from_utf16_lossy(&label[..len])
                } else {
                    String::new()
                };

                drives.push((drive_root, label_str));
            }
        }
        drives
    }

    #[cfg(not(windows))]
    pub fn get_logical_drives() -> Vec<(String, String)> {
        vec![("/".to_string(), "Root".to_string())]
    }
}

#[async_trait]
impl DeviceBackend for LocalBackend {
    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        let entries = fs::read_dir(path)?;
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let full_path = entry.path().to_string_lossy().to_string();
            let is_hidden = name.starts_with('.');
            let modified = metadata.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
            result.push(FileEntry {
                name,
                full_path,
                size: metadata.len(),
                modified,
                is_dir: metadata.is_dir(),
                is_hidden,
            });
        }
        Ok(result)
    }

    async fn stat(&self, path: &str) -> Result<FileEntry> {
        let p = Path::new(path);
        let metadata = fs::metadata(p)?;
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        let is_hidden = name.starts_with('.');
        let modified = metadata.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
        Ok(FileEntry {
            name,
            full_path: path.to_string(),
            size: metadata.len(),
            modified,
            is_dir: metadata.is_dir(),
            is_hidden,
        })
    }

    async fn push(&self, _local_path: &str, _remote_path: &str) -> Result<()> {
        Ok(())
    }

    async fn pull(&self, _remote_path: &str, _local_path: &str) -> Result<()> {
        Ok(())
    }

    async fn mkdir(&self, path: &str) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        if Path::new(path).is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> Result<()> {
        fs::rename(old_path, new_path)?;
        Ok(())
    }

    async fn refresh_media(&self, _path: &str) -> Result<()> {
        // Windows usually handles this automatically or via shell notify,
        // no-op for now to satisfy trait.
        Ok(())
    }
}
