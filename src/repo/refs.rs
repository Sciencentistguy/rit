use std::io::Write;

use crate::digest::Digest;
use crate::lock::LockedFile;
use crate::Result;

use color_eyre::eyre::eyre;

impl super::Repo {
    /// Updates the value of HEAD to oid
    pub fn set_head(&self, oid: &Digest) -> Result<()> {
        // File::create(root.join(".git/HEAD"))?.write_all(commit.get_oid().lower_hex().as_bytes())?;
        // let mut head = File::create(&self.head_path)?;
        let mut head = LockedFile::try_aquire(&self.head_path)?
            .ok_or_else(|| eyre!("Could not aquire lock file `{:?}.lock`", self.head_path))?;
        writeln!(&mut head, "{oid:x}")?;
        Ok(())
    }
}
