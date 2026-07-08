use async_trait::async_trait;
use crate::traits::DeviceBackend;
use adb_explorer_common::models::FileEntry;
use anyhow::Result;

pub struct MockBackend;

#[async_trait]
impl DeviceBackend for MockBackend {
    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        Ok(vec![
            FileEntry {
                name: "folder1".to_string(),
                full_path: format!("{}/folder1", path),
                size: 0,
                modified: None,
                is_dir: true,
                is_hidden: false,
            },
            FileEntry {
                name: "file1.txt".to_string(),
                full_path: format!("{}/file1.txt", path),
                size: 1024,
                modified: None,
                is_dir: false,
                is_hidden: false,
            },
        ])
    }

    async fn stat(&self, path: &str) -> Result<FileEntry> {
        Ok(FileEntry {
            name: path.split('/').last().unwrap_or(path).to_string(),
            full_path: path.to_string(),
            size: 0,
            modified: None,
            is_dir: true,
            is_hidden: false,
        })
    }

    async fn push(&self, _local_path: &str, _remote_path: &str) -> Result<()> {
        Ok(())
    }

    async fn pull(&self, _remote_path: &str, _local_path: &str) -> Result<()> {
        Ok(())
    }

    async fn mkdir(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    async fn delete(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> Result<()> {
        Ok(())
    }

    async fn refresh_media(&self, _path: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_list_dir() {
        let backend = MockBackend;
        let files = backend.list_dir("/test").await.unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "folder1");
        assert!(files[0].is_dir);
    }

    #[tokio::test]
    async fn test_mock_stat() {
        let backend = MockBackend;
        let file = backend.stat("/test/file1.txt").await.unwrap();
        assert_eq!(file.name, "file1.txt");
        assert_eq!(file.full_path, "/test/file1.txt");
    }
}
