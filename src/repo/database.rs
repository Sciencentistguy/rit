use crate::blob::Blob;
use crate::commit::Commit;
use crate::digest::Digest;
use crate::storable::DatabaseObject;
use crate::storable::Storable;
use crate::tree::Tree;
use crate::util;
use crate::Result;

use std::io::Read;
use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use tracing::*;

pub struct Database {
    database_root: Utf8PathBuf,
}

impl Database {
    pub fn new(git_folder: impl AsRef<Utf8Path>) -> Self {
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

    fn object_path(&self, oid: &Digest) -> Utf8PathBuf {
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

    fn load(&self, oid: &Digest) -> Result<LoadedItem> {
        let mut bytes = self.read(oid)?;

        let space_idx = bytes.iter().position(|&b| b == b' ').unwrap();
        let nul_idx = bytes.iter().position(|&b| b == b'\0').unwrap();
        let r#type = &bytes[..space_idx];
        debug_assert!({
            let len = &bytes[space_idx + 1..nul_idx];
            let len = std::str::from_utf8(len)?;
            len.parse::<usize>()? > 0
        });

        let content_start = nul_idx + 1;

        match r#type {
            b"blob" => {
                bytes.drain(0..content_start);
                assert!(bytes.starts_with(b"blob"));
                Ok(LoadedItem::Blob(Blob::new(bytes)))
            }
            b"tree" => {
                let bytes = &bytes[content_start..];
                Ok(LoadedItem::Tree(Tree::parse(bytes)?))
            }
            b"commit" => {
                let bytes = &bytes[content_start..];
                Ok(LoadedItem::Commit(Commit::parse(bytes)?))
                // commit
            }
            _ => unreachable!("Unexpected object type: {}", std::str::from_utf8(r#type)?),
        }
    }
}

pub enum LoadedItem {
    Commit(Commit),
    Tree(Tree),
    Blob(Blob),
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use crate::{
        repo::Repo,
        test::{COMMIT_EMAIL, COMMIT_NAME},
    };

    use super::*;

    #[test]
    fn works() -> Result<()> {
        std::env::set_var("RIT_AUTHOR_NAME", COMMIT_NAME);
        std::env::set_var("RIT_AUTHOR_EMAIL", COMMIT_EMAIL);

        let root = TempDir::new("")?;
        let root = root.path();
        let root = Utf8Path::from_path(root).unwrap();

        Repo::init(root)?;
        let mut repo = Repo::open(root.to_owned())?;

        crate::create_test_files!(root, ["file1"]);

        repo.add(&[".".into()])?;

        let oid = repo.commit("test")?;

        let bytes = repo.database.read(&oid)?;

        let string = std::str::from_utf8(&bytes)?;

        println!("{string}");

        Ok(())
    }
}
