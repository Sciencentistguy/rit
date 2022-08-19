use std::fs::File;
use std::io::Write;

use camino::Utf8Path;
use color_eyre::eyre::eyre;

use crate::digest::Digest;
use crate::Result;

/// Contains all characters that cannot appear in a ref name.
///
/// In git, the character `'*'` is allowed in ref names if the environment variable
/// `REFNAME_REFSPEC_PATTERN` is set. Rit does not allow this, so `'*'` appears in this array.
///
/// See: <https://github.com/git/git/blob/795ea8776befc95ea2becd8020c7a284677b4161/refs.c#L48-L57>
const DISALLOWED_CHARACTERS: [char; 40] = [
    '\x01', '\x02', '\x03', '\x04', '\x05', '\x06', '\x07', '\x08', '\t', '\n', '\x0b', '\x0c',
    '\r', '\x0e', '\x0f', '\x10', '\x11', '\x12', '\x13', '\x14', '\x15', '\x16', '\x17', '\x18',
    '\x19', '\x1a', '\x1b', '\x1c', '\x1d', '\x1e', '\x1f', ' ', '*', ':', '?', '[', '\\', '^',
    '~', '\x7f',
];

/// Check whether a string is a valid ref name.
///
/// This is not a port of `check_refname_component` from git, but is based on the documentation for
/// that function.
///
/// Disallowed paths are any path where:
///
/// - it (or any path component) begins with `'.'`
/// - it contains double dots `".."`
/// - it contains ASCII control characters
/// - it contains ':', '?', '[', '\', '^', '~', SP, or TAB anywhere
/// - it contains `'*'`
/// - it ends with `'/'`
/// - it ends with `".lock"`
/// - it contains `"@{"`
///
/// See: <https://github.com/git/git/blob/795ea8776befc95ea2becd8020c7a284677b4161/refs.c#L59-L77>
fn is_valid_ref_name(name: &str) -> bool {
    !((name.chars().any(|c| DISALLOWED_CHARACTERS.contains(&c)))
        || name.starts_with('.')
        || name.contains("/.")
        || name.contains("..")
        || name.ends_with('/')
        || name.ends_with(".lock")
        || name.contains("@{"))
}

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

    fn update_ref_file(&self, path: &Utf8Path, oid: &Digest) -> Result<()> {
        dbg!(path);
        let mut file = File::create(path)?;
        writeln!(&mut file, "{oid:x}")?;
        Ok(())
    }
}
