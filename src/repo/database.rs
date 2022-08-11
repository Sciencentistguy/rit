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
    pub database_root: Utf8PathBuf,
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

    pub fn read_to_vec(&self, oid: &Digest) -> Result<Vec<u8>> {
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

    pub fn load(&self, oid: &Digest) -> Result<LoadedItem> {
        let mut bytes = self.read_to_vec(oid)?;

        let space_idx = memchr::memchr(b' ', &bytes).unwrap();
        let nul_idx = memchr::memchr(b'\0', &bytes).unwrap();
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
                Ok(LoadedItem::Blob(Blob::new(bytes)))
            }
            b"tree" => {
                let bytes = &bytes[content_start..];
                let root = self.database_root.parent().unwrap().parent().unwrap();
                Ok(LoadedItem::Tree(Tree::parse(bytes, root, self)?))
            }
            b"commit" => {
                let bytes = &bytes[content_start..];
                Ok(LoadedItem::Commit(Commit::parse(bytes)?))
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

impl LoadedItem {
    pub fn into_commit(self) -> Option<Commit> {
        if let Self::Commit(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn into_tree(self) -> Option<Tree> {
        if let Self::Tree(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn into_blob(self) -> Option<Blob> {
        if let Self::Blob(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the loaded item is [`Tree`].
    ///
    /// [`Tree`]: LoadedItem::Tree
    #[must_use]
    pub fn is_tree(&self) -> bool {
        matches!(self, Self::Tree(..))
    }

    /// Returns `true` if the loaded item is [`Commit`].
    ///
    /// [`Commit`]: LoadedItem::Commit
    #[must_use]
    pub fn is_commit(&self) -> bool {
        matches!(self, Self::Commit(..))
    }

    /// Returns `true` if the loaded item is [`Blob`].
    ///
    /// [`Blob`]: LoadedItem::Blob
    #[must_use]
    pub fn is_blob(&self) -> bool {
        matches!(self, Self::Blob(..))
    }

    pub fn as_tree(&self) -> Option<&Tree> {
        if let Self::Tree(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_commit(&self) -> Option<&Commit> {
        if let Self::Commit(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_blob(&self) -> Option<&Blob> {
        if let Self::Blob(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use tempdir::TempDir;

    use crate::{
        repo::Repo,
        test::{COMMIT_EMAIL, COMMIT_NAME},
    };

    use super::*;

    #[test]
    /// Create a dir, add a file, commit it. Then use git cat-file to get the tree_id of that
    /// commit. Seperately, parse that commit using Database::load. The tree_id should be the same.
    /// Then load the tree and check that the name of the first entry is as expected.
    /// Load the blob from that file oid, and that should match the contents of the file (known)
    fn test_database_load() -> Result<()> {
        std::env::set_var("RIT_AUTHOR_NAME", COMMIT_NAME);
        std::env::set_var("RIT_AUTHOR_EMAIL", COMMIT_EMAIL);

        let root = TempDir::new("")?;
        let root = root.path();
        let root = Utf8Path::from_path(root).unwrap();

        Repo::init(root)?;
        let mut repo = Repo::open(root.to_owned())?;

        crate::create_test_files!(root, ["file1"]);

        repo.add(&[".".into()])?;

        let commit_id = repo.commit("test")?;

        let commit = repo.database.load(&commit_id)?;
        let commit = commit.as_commit().unwrap();

        let commit_text = crate::test::git_cat_file(root.as_std_path(), &commit_id)?;

        let tid = commit_text.lines().next().unwrap().split_at(5).1;
        let tid = Digest::from_str(tid)?;

        assert_eq!(commit.tree_id(), &tid);

        let tree = repo.database.load(&tid)?;
        let tree = tree.as_tree().unwrap();

        let (name, entry) = tree.entries().iter().next().unwrap();

        assert_eq!(name, "file1");

        let blob_id = entry.oid().unwrap();

        let blob = repo.database.load(blob_id)?;
        let blob = blob.as_blob().unwrap();

        let expected = crate::test_file_contents!("file1");

        assert_eq!(blob.data(), expected.as_bytes());

        Ok(())
    }
}
