use std::{collections::BTreeMap, ffi::CStr, str};

use crate::{
    digest::Digest,
    filemode::FileMode,
    index::IndexEntry,
    repo::{database::Database, Repo},
    tree::Tree,
    Result,
};

use super::TreeEntry;

use camino::Utf8Path;
use color_eyre::eyre::{eyre, Context};
use once_cell::sync::OnceCell;

impl super::Tree {
    pub fn parse(mut bytes: &[u8], root: &Utf8Path, database: &Database) -> Result<Self> {
        let mut entries: BTreeMap<String, TreeEntry> = Default::default();

        while let Some(null_idx) = bytes.iter().position(|&c| c == b'\0') {
            let line = &bytes[..null_idx + 21];
            bytes = &bytes[null_idx + 21..];

            let (name, entry) = TreeEntry::parse(line, root, database)?;
            entries.insert(name, entry);
        }

        Ok(Self {
            entries,
            oid: OnceCell::new(),
        })
    }
}

impl super::TreeEntry {
    /// Parses an entry from the tree. Lines are of the form
    /// `<mode> <name>\0<oid>`
    fn parse(line: &[u8], prefix: &Utf8Path, database: &Database) -> Result<(String, Self)> {
        let mode_len = line.iter().position(|&b| b == b' ').unwrap();

        let (mode, line) = line.split_at(mode_len);
        let mode = str::from_utf8(mode)?.trim();
        let mode = u32::from_str_radix(mode, 8)?;

        // mode_t is a u16 bits on macOS
        #[cfg(target_os = "macos")]
        let mode: libc::mode_t = mode as _;

        let mode = FileMode::from(mode);

        let line = &line[1..];
        let name = unsafe {
            // Assert should never trip, as this is enforced by the loop condition in
            // `Tree::parse`.
            assert!(line.contains(&b'\0'), "Line must contain null byte");

            // Safety: line contains null byte.
            CStr::from_ptr(line.as_ptr().cast())
        }
        .to_str()
        .wrap_err(eyre!("invalid utf-8 in tree entry name"))?
        .to_owned();

        let (_, oid) = line.split_at(name.len() + 1 /* for null byte*/);

        let oid = Digest(oid.try_into().unwrap());

        // let path = Utf8Path::new(&name);
        let path = prefix.join(&name);

        if path.is_dir() {
            let bytes = database.read_uncompressed(&oid)?;
            let nul_idx = memchr::memchr(b'\0', &bytes).unwrap();
            let bytes = &bytes[nul_idx + 1..];
            let subtree = Tree::parse(bytes, &prefix.join(&name), database)?;
            Ok((
                name.clone(),
                TreeEntry::Directory {
                    tree: subtree,
                    name,
                },
            ))
        } else if path.exists() {
            let stat = Repo::stat_file(&path)?.expect("File should exist");
            Ok((
                name.clone(),
                Self::File(IndexEntry::new(Utf8Path::new(&name), &oid, stat)?),
            ))
        } else {
            Ok((name.clone(), Self::IncompleteFile { oid, name, mode }))
        }
    }
}
