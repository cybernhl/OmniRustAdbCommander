use async_trait::async_trait;
use crate::traits::DeviceBackend;
use adb_explorer_common::models::FileEntry;
use anyhow::{Result, anyhow};
use radb::AdbClient;

pub struct AdbBackend {
    serial: String,
    adb_server_addr: String,
}

impl AdbBackend {
    pub fn new(serial: String, adb_server_addr: String) -> Self {
        Self {
            serial,
            adb_server_addr,
        }
    }

    async fn get_device(&self) -> Result<radb::AdbDevice<String>> {
        let mut client = AdbClient::connect(self.adb_server_addr.clone()).await?;
        let devices = client.list_devices().await?;
        for device in devices {
            if device.serial.as_deref() == Some(&self.serial) {
                // Return a new AdbDevice with the same serial and our known address
                return Ok(radb::AdbDevice::new(self.serial.clone(), self.adb_server_addr.clone()));
            }
        }
        Err(anyhow!("Device {} not found", self.serial))
    }
}

#[async_trait]
impl DeviceBackend for AdbBackend {
    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        let mut device = self.get_device().await?;
        let files = device.list(path).await?;

        let base_path = path.trim_end_matches('/');

        Ok(files.into_iter().map(|f| {
            let full_path = format!("{}/{}", base_path, f.path);
            FileEntry {
                name: f.path.clone(),
                full_path,
                size: f.size as u64,
                modified: Some(f.mtime as u64),
                is_dir: (f.mode & 0o170000) == 0o040000,
                is_hidden: f.path.starts_with('.'),
            }
        }).collect())
    }

    async fn stat(&self, path: &str) -> Result<FileEntry> {
        let mut device = self.get_device().await?;
        let f = device.stat(path).await?;

        let name = path.split('/').last().unwrap_or("").to_string();
        let is_hidden = name.starts_with('.');

        Ok(FileEntry {
            name,
            full_path: path.to_string(),
            size: f.size as u64,
            modified: Some(f.mtime as u64),
            is_dir: (f.mode & 0o170000) == 0o040000,
            is_hidden,
        })
    }

    async fn push(&self, local_path: &str, remote_path: &str) -> Result<()> {
        let mut device = self.get_device().await?;
        device.push(local_path, remote_path).await?;
        Ok(())
    }

    async fn pull(&self, remote_path: &str, local_path: &str) -> Result<()> {
        let mut device = self.get_device().await?;
        device.pull(remote_path, &std::path::PathBuf::from(local_path)).await?;
        Ok(())
    }

    async fn mkdir(&self, path: &str) -> Result<()> {
        let mut device = self.get_device().await?;
        device.shell(format!("mkdir -p '{}'", path)).await?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let mut device = self.get_device().await?;
        device.shell(format!("rm -rf '{}'", path)).await?;
        Ok(())
    }

    async fn rename(&self, old_path: &str, new_path: &str) -> Result<()> {
        let mut device = self.get_device().await?;
        device.shell(format!("mv '{}' '{}'", old_path, new_path)).await?;
        Ok(())
    }

    async fn refresh_media(&self, path: &str) -> Result<()> {
        let mut device = self.get_device().await?;
        // Trigger media scanner for the file
        // Different Android versions might need different commands, this is the most common one.
        device.shell(format!("am broadcast -a android.intent.action.MEDIA_SCANNER_SCAN_FILE -d 'file://{}'", path)).await?;
        Ok(())
    }
}
