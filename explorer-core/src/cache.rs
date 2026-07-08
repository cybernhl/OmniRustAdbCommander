use std::collections::HashMap;
use adb_explorer_common::models::FileEntry;
use std::time::SystemTime;

pub struct DirectoryCache {
    entries: HashMap<String, (Vec<FileEntry>, SystemTime)>,
}

impl DirectoryCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, path: &str) -> Option<&Vec<FileEntry>> {
        self.entries.get(path).map(|(entries, _)| entries)
    }

    pub fn insert(&mut self, path: String, entries: Vec<FileEntry>) {
        self.entries.insert(path, (entries, SystemTime::now()));
    }

    pub fn invalidate(&mut self, path: &str) {
        self.entries.remove(path);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
