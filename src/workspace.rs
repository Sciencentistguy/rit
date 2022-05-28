use crate::Result;

use std::path::{Path, PathBuf};

pub struct Workspace {
    root_path: PathBuf,
}

impl Workspace {
    const IGNORE: [&'static str; 1] = [".git"];

    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            root_path: root_path.as_ref().canonicalize().unwrap(),
        }
    }

    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for x in self.root_path.read_dir()? {
            let x = x?;

            let path = x.path();
            if let Some(true) = path
                .file_name()
                .expect("not dealing with root")
                .to_str()
                .map(|x| Self::IGNORE.contains(&x))
            {
                // Filename is in IGNORE
                continue;
            }
            files.push(path);
        }

        Ok(files)
    }
}
