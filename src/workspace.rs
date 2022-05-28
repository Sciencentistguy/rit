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

    pub fn list_files(&self) -> Result<impl Iterator<Item = PathBuf>> {
        Ok(self.root_path.read_dir()?.filter_map(|x| {
            let x = x.ok()?;

            let path = x.path();
            match path
                .file_name()
                .expect("not dealing with root")
                .to_str()
                .map(|x| Self::IGNORE.contains(&x))
            {
                // Filename is in IGNORE
                Some(true) => None,
                //Filename is not in IGNORE (or is invalid UTF-8, which means not in IGNORE)
                _ => Some(path),
            }
        }))
    }
}
