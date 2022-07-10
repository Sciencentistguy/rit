use crate::storable::Storable;
use crate::util;
use crate::Result;

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use color_eyre::eyre::eyre;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use tracing::*;

pub struct Database {
    database_root: PathBuf,
}

impl Database {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        let mut database_root = root_path.as_ref().canonicalize().unwrap();
        database_root.push(".git");
        database_root.push("objects");
        Self { database_root }
    }

    pub fn store(&self, obj: &impl Storable) -> Result<()> {
        trace!(oid=?obj.get_oid(), "Writing object to database");
        let content = obj.formatted();

        let object_path = {
            let mut x = self.database_root.to_owned();
            let oid = obj.get_oid().to_hex();
            let (prefix, suffix) = oid.split_at(2);
            debug_assert_eq!(prefix.len(), 2);
            x.push(prefix);
            x.push(suffix);
            x
        };

        if object_path.exists() {
            return Ok(());
        }

        let dirname = object_path
            .parent()
            .ok_or_else(|| eyre!("object had no parent"))?;

        let temp_path = dirname.join(util::tmp_file_name());

        if !dirname.is_dir() {
            std::fs::create_dir_all(dirname)?;
        }

        let mut file = std::fs::File::create(&temp_path)?;

        let mut e = ZlibEncoder::new(Vec::with_capacity(content.len()), Compression::fast());
        e.write_all(content)?;
        let compressed_bytes = e.finish()?;

        file.write_all(&compressed_bytes)?;

        drop(file);

        std::fs::rename(temp_path, object_path)?;

        Ok(())
    }
}
