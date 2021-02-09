use std::{collections::HashMap, fs::read_link, path::PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct SpaceFolder {
    pub entry: String,
    folders: Vec<PathBuf>,
    rooms: Vec<PathBuf>,
    symlinks: HashMap<PathBuf, PathBuf>,
}

impl SpaceFolder {
    fn get_folders(&mut self) -> color_eyre::Result<()> {
        for entry in WalkDir::new(&self.entry)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name == "metadata.yml" || file_name == "metadata.yaml" {
                        self.folders.push(entry.into_path().clone());
                        continue;
                    }

                    if file_name.starts_with("!") {
                        self.rooms.push(entry.into_path().clone());
                        continue;
                    }
                }
            }
            if entry.path_is_symlink() {
                let full_path = read_link(entry.path())?;
                self.symlinks.insert(entry.into_path().clone(), full_path);
                continue;
            }
        }
        Ok(())
    }
}
