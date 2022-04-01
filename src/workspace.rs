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

    pub fn list_files(&self) -> Result<impl Iterator<Item = String>> {
        Ok(self.root_path.read_dir()?.filter_map(|x| {
            let x = x.ok()?;

            // TODO proper error handling
            let filename = match x.file_name().into_string() {
                Ok(x) => x,
                Err(e) => panic!("non-utf8 path name waa {:?}", e),
            };

            if Self::IGNORE.contains(&&*filename) {
                None
            } else {
                Some(filename)
            }
        }))
    }
}
