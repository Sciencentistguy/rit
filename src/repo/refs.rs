use std::fs::File;
use std::io::Write;

use camino::Utf8Path;
use color_eyre::eyre::eyre;

use crate::digest::Digest;
use crate::Result;
use crate::revision::is_valid_ref_name;

impl super::Repo {
    /// Updates the value of HEAD to oid
    pub fn set_head(&mut self, oid: &Digest) -> Result<()> {
        self.update_ref_file(&self.head_path, oid)
    }

    pub fn create_branch(&mut self, name: &str) -> Result<()> {
        if !is_valid_ref_name(name) {
            return Err(eyre!("Invalid ref name: {}", name));
        }

        let path = self.heads_path.join(name);

        if path.exists() {
            return Err(eyre!("Branch already exists: {}", name));
        }

        self.update_ref_file(&path, &self.read_head()?.unwrap())
    }

    /// Set the value of a ref file to the specified oid. 
    ///
    /// This function does not use git locks. This is a design decision. This creates a possible
    /// issue when multiple processes (realistically, git and rit) are contending a head file. The
    /// solution to this is to Just Not run rit while a git process is running.
    fn update_ref_file(&self, path: &Utf8Path, oid: &Digest) -> Result<()> {
        dbg!(path);
        let mut file = File::create(path)?;
        writeln!(&mut file, "{oid:x}")?;
        Ok(())
    }
}
