use std::io::Write;
use std::path::{Path, PathBuf};

use crate::digest::Digest;
use crate::lock::LockedFile;
use crate::Result;

use color_eyre::eyre::{eyre, Context};

pub struct Refs {
    git_dir: PathBuf,
    head_path: PathBuf,
}

impl Refs {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        let git_dir = root_path.as_ref().canonicalize().unwrap().join(".git");
        let head_path = git_dir.join("HEAD");
        Self { git_dir, head_path }
    }

    /// Updates the value of HEAD to oid
    pub fn set_head(&self, oid: &Digest) -> Result<()> {
        // File::create(root.join(".git/HEAD"))?.write_all(commit.get_oid().lower_hex().as_bytes())?;
        // let mut head = File::create(&self.head_path)?;
        let mut head = LockedFile::try_aquire(&self.head_path)?
            .ok_or_else(|| eyre!("Could not aquire lock file `{:?}.lock`", &*self.head_path))?;
        writeln!(&mut head, "{oid:x}")?;
        Ok(())
    }

    pub fn read_head(&self) -> Result<Option<Digest>> {
        if !self.head_path.is_file() {
            return Ok(None);
        }
        let mut oid = Digest::default();
        let read = std::fs::read_to_string(&self.head_path)?;
        let x = hex::decode(read.trim().as_bytes()).wrap_err("Invalid value in HEAD")?;
        oid.0.copy_from_slice(&x);
        Ok(Some(oid))
    }
}
