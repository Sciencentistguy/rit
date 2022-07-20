use crate::digest::Digest;
use crate::storable::Storable;
use crate::storable::DatabaseObject;
use crate::util;
use crate::Result;

use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use color_eyre::eyre::eyre;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use tracing::*;

pub struct Database {
    database_root: PathBuf,
}

impl Database {
    pub fn new(git_folder: impl AsRef<Path>) -> Self {
        Self {
            database_root: git_folder.as_ref().join("objects"),
        }
    }

    pub fn store<T: Storable>(&self, obj: &DatabaseObject<T>) -> Result<()> {
        trace!(oid=?obj.oid(), "Writing object to database");
        let content = obj.formatted();

        let object_path = self.object_path(obj.oid());

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

    fn object_path(&self, oid: &Digest) -> PathBuf {
        let mut x = self.database_root.to_owned();
        let oid = oid.to_hex();
        let (prefix, suffix) = oid.split_at(2);
        debug_assert_eq!(prefix.len(), 2);
        x.push(prefix);
        x.push(suffix);
        x
    }

    pub fn exists(&self, oid: &Digest) -> bool {
        self.object_path(oid).exists()
    }

    pub fn read(&self, oid: &Digest) -> Result<Vec<u8>> {
        trace!(object=%oid.to_hex(), "Reading object from database");

        let object_path = self.object_path(oid);

        if !object_path.exists() {
            return Err(eyre!("object not found in database: {:x}", oid));
        }

        let compressed = std::fs::read(object_path)?;

        let mut d = ZlibDecoder::new(&*compressed);

        let mut decompressed = Vec::new();

        let _ = d.read_to_end(&mut decompressed)?;

        Ok(decompressed)
    }
}
