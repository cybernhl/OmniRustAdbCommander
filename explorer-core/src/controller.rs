use std::sync::Arc;
use adb_explorer_backend::traits::DeviceBackend;
use adb_explorer_common::models::FileEntry;
use anyhow::Result;
use crate::selection::SelectionModel;
use crate::cache::DirectoryCache;

pub struct ExplorerController {
    backend: Arc<dyn DeviceBackend>,
    current_path: String,
    selection: SelectionModel,
    cache: DirectoryCache,
}

impl ExplorerController {
    pub fn new(backend: Arc<dyn DeviceBackend>) -> Self {
        Self {
            backend,
            current_path: "/".to_string(),
            selection: SelectionModel::new(),
            cache: DirectoryCache::new(),
        }
    }

    pub async fn list_current_dir(&mut self, force_refresh: bool) -> Result<Vec<FileEntry>> {
        if !force_refresh {
            if let Some(entries) = self.cache.get(&self.current_path) {
                return Ok(entries.clone());
            }
        }

        let entries = self.backend.list_dir(&self.current_path).await?;
        self.cache.insert(self.current_path.clone(), entries.clone());
        Ok(entries)
    }

    pub fn set_path(&mut self, path: String) {
        self.current_path = path;
        // Optional: clear selection when changing directory
        // self.selection.clear();
    }

    pub fn current_path(&self) -> &str {
        &self.current_path
    }

    pub fn selection(&mut self) -> &mut SelectionModel {
        &mut self.selection
    }

    pub fn refresh(&mut self) {
        self.cache.invalidate(&self.current_path);
    }
}
