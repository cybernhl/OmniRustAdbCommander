use std::collections::HashSet;
use std::path::PathBuf;

pub struct SelectionModel {
    selected_paths: HashSet<PathBuf>,
}

impl SelectionModel {
    pub fn new() -> Self {
        Self {
            selected_paths: HashSet::new(),
        }
    }

    pub fn select(&mut self, path: PathBuf) {
        self.selected_paths.insert(path);
    }

    pub fn deselect(&mut self, path: &PathBuf) {
        self.selected_paths.remove(path);
    }

    pub fn toggle(&mut self, path: PathBuf) {
        if self.selected_paths.contains(&path) {
            self.selected_paths.remove(&path);
        } else {
            self.selected_paths.insert(path);
        }
    }

    pub fn is_selected(&self, path: &PathBuf) -> bool {
        self.selected_paths.contains(path)
    }

    pub fn clear(&mut self) {
        self.selected_paths.clear();
    }

    pub fn selected_count(&self) -> usize {
        self.selected_paths.len()
    }
}
