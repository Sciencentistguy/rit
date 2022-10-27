use std::fs::File;
use std::io::Write;
use std::str::FromStr;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use tracing::trace;

use crate::digest::Digest;
use crate::revision::is_valid_ref_name;
use crate::Result;

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
        trace!(%path, ?oid, "Updating ref");
        let mut file = File::create(path)?;
        writeln!(&mut file, "{oid:x}")?;
        Ok(())
    }

    pub fn read_ref(&self, name: &str) -> Result<Option<Digest>> {
        if let Some(path) = self.path_for_ref(name) {
            let string = std::fs::read_to_string(path)?;
            let string = string.trim();
            dbg!(string);
            let oid = Digest::from_str(string)?;

            return Ok(Some(oid));
        }

        if !name.chars().all(|c| c.is_ascii_hexdigit()) {
            // Definitely not an oid fragment
            return Ok(None);
        }

        let mut candidates = self.database.prefix_match(name)?;

        match candidates.len() {
            0 => Ok(None),
            1 => {
                let oid = candidates.pop().unwrap();
                if self.database.load(&oid)?.is_commit() {
                    Ok(Some(oid))
                } else {
                    Err(eyre!("Refname was a valid sha1 fragment, but pointed to something other than a commit"))
                }
            }
            _ => {
                eprintln!("Too many candidates for prefix {}:", name);

                for candidate in candidates {
                    eprintln!("\t{:x}", candidate);
                }

                Err(eyre!("Too many candidates for prefix {}:", name))
            }
        }
    }

    fn path_for_ref(&self, name: &str) -> Option<Utf8PathBuf> {
        if name == "HEAD" {
            return Some(self.head_path.clone());
        }

        let x = self.refs_path.join(name);
        if x.exists() {
            return Some(x);
        }

        let x = self.heads_path.join(name);
        if x.exists() {
            return Some(x);
        }

        None
    }
}
