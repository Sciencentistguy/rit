use camino::Utf8PathBuf;
use color_eyre::eyre::{eyre, Context};
use tracing::trace;

use crate::{blob::Blob, storable::DatabaseObject, Result};

impl super::Repo {
    /// Add paths to the index.
    /// 
    /// if `paths` is empty, do nothing
    pub fn add(&mut self, paths: &[Utf8PathBuf]) -> Result<()> {
        for path in paths {
            trace!(?path, "Adding file to repo");
            if !self.dir.join(path).exists() {
                return Err(eyre!("Path does not exist: {}", path));
            }
            let paths = self.list_files(path)?;
            for path in paths {
                let path = if path.has_root() {
                    path.strip_prefix(&self.dir)
                        .wrap_err(format!("Path: {:?}", path))?
                } else {
                    &path
                };
                trace!(?path, "Adding file");
                let abs_path = self.dir.join(&path);

                let data = std::fs::read(&abs_path)
                    .wrap_err(format!("Failed to read file: {}", abs_path))?;
                let stat = Self::stat_file(&abs_path)?.unwrap();

                let blob = Blob::new(data);
                let blob = DatabaseObject::new(&blob);
                self.database.store(&blob)?;
                self.index.add(path, blob.oid(), stat);
            }
        }
        self.index.flush()?;

        Ok(())
    }

    pub fn add_all(&mut self) -> Result<()> {
        self.add(&[".".into()])
    }
}
