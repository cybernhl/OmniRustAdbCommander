use adb_explorer_common::models::FileEntry;

pub trait FilterRule: Send + Sync {
    fn filter(&self, entry: &FileEntry) -> bool;
}

pub struct ExtensionFilter {
    pub extensions: Vec<String>,
}

impl FilterRule for ExtensionFilter {
    fn filter(&self, entry: &FileEntry) -> bool {
        if entry.is_dir { return true; }
        self.extensions.iter().any(|ext| entry.name.ends_with(ext))
    }
}

pub struct HiddenFileFilter {
    pub show_hidden: bool,
}

impl FilterRule for HiddenFileFilter {
    fn filter(&self, entry: &FileEntry) -> bool {
        self.show_hidden || !entry.is_hidden
    }
}
