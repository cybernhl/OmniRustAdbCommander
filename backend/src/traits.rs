use async_trait::async_trait;
use adb_explorer_common::models::FileEntry;
use anyhow::Result;

#[async_trait]
pub trait DeviceBackend: Send + Sync {
    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>>;
    async fn stat(&self, path: &str) -> Result<FileEntry>;
    async fn push(&self, local_path: &str, remote_path: &str) -> Result<()>;
    async fn pull(&self, remote_path: &str, local_path: &str) -> Result<()>;
    async fn mkdir(&self, path: &str) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn rename(&self, old_path: &str, new_path: &str) -> Result<()>;
    async fn refresh_media(&self, path: &str) -> Result<()>;
}
