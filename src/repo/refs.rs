use std::fs::File;
use std::io::Write;
use std::str::FromStr;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use tap::Tap;
use tracing::trace;

use crate::digest::Digest;
use crate::repo::database::LoadedItem;
use crate::revision::is_valid_ref_name;
use crate::Result;

impl super::Repo {
    /// Updates the value of HEAD to oid
    pub fn set_head(&mut self, oid: &Digest) -> Result<()> {
        self.update_ref_file(&self.head_path, oid)
    }

    pub fn create_branch(&mut self, name: &str, target: &Digest) -> Result<()> {
        if !is_valid_ref_name(name) {
            return Err(eyre!("Invalid ref name: {}", name));
        }

        let path = self.heads_path.join(name);

        if path.exists() {
            return Err(eyre!("Branch already exists: {}", name));
        }

        self.update_ref_file(&path, target)
    }

    /// Set the value of a ref file to the specified oid.
    ///
    /// This function does not use git locks. This creates a possible issue when multiple processes
    /// (realistically, git and rit) are contending a head file. The solution to this is to Just
    /// Not run rit while a git process is running.
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

            if string.starts_with("ref: refs/") {
                // This is a symbolic ref
                // See: https://git-scm.com/docs/git-symbolic-ref
                let target = string.trim_start_matches("ref: refs/");
                return self.read_ref(target);
            }

            let oid = Digest::from_str(string)?;
            return Ok(Some(oid));
        }

        if !name.chars().all(|c| c.is_ascii_hexdigit()) {
            // Definitely not an oid fragment
            return Ok(None);
        }

        let candidates = self.database.prefix_match(name)?;

        match &candidates[..] {
            [] => Ok(None),
            [oid] => Ok(Some(oid.clone())),
            candidates => {
                // TODO: this should not explode. Make it return a real error type that captures
                // this information
                eprintln!("Too many candidates for prefix {}:", name);

                for candidate in candidates {
                    eprint!("\t{candidate:x}");
                    let loaded = self.database.load(candidate)?;
                    let kind = loaded.kind();
                    eprint!(" {kind}");
                    if let LoadedItem::Commit(commit) = loaded {
                        let date = commit.commit_date();
                        let first_line = commit
                            .message()
                            .split_once('\n')
                            .map(|x| x.0)
                            .unwrap_or(commit.message());
                        eprint!(" {date}: {first_line}");
                    }
                    eprintln!()
                }

                std::process::exit(1);
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

        let x = x.tap_mut(|x| {
            x.clear();
            x.push(&self.heads_path);
            x.push(name);
        });

        if x.exists() {
            return Some(x);
        }

        None
    }
}
