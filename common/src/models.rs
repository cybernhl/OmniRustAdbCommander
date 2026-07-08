use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub full_path: String,
    pub size: u64,
    pub modified: Option<u64>, // Unix timestamp
    pub is_dir: bool,
    pub is_hidden: bool,
}
