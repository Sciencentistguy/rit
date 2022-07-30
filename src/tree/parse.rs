use std::{collections::BTreeMap, ffi::CStr, str};

use crate::{digest::Digest, filemode::FileMode, Result};

use super::TreeEntry;

use color_eyre::eyre::{eyre, Context};
use once_cell::sync::OnceCell;

impl super::Tree {
    pub fn parse(mut bytes: &[u8]) -> Result<Self> {
        let mut entries: BTreeMap<String, TreeEntry> = Default::default();

        while let Some(null_idx) = bytes.iter().position(|&c| c == b'\0') {
            let line = &bytes[..null_idx + 21];
            bytes = &bytes[null_idx + 21..];

            let (name, entry) = TreeEntry::parse(line)?;
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
    fn parse(line: &[u8]) -> Result<(String, Self)> {
        // const MODE_LEN: usize = 6;
        let mode_len = line.iter().position(|&b| b == b' ').unwrap();

        let (mode, line) = line.split_at(mode_len);
        let mode = str::from_utf8(mode)?.trim();
        let mode = FileMode(u32::from_str_radix(mode, 8)?);

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

        Ok((name.clone(), Self::Database { oid, name, mode }))
    }
}
