use std::collections::BTreeMap;

use crate::{
    index::IndexEntry,
    repo::{database::Database, Repo},
    tree::Tree,
    Result,
};

use super::TreeEntry;

use camino::Utf8Path;
use color_eyre::eyre::eyre;
use once_cell::sync::OnceCell;

mod nom {
    use bstr::ByteSlice;
    use nom::Parser;

    use crate::{digest::Digest, filemode::FileMode};

    pub type Input<'a> = &'a [u8];
    pub type Result<'a, O> = nom::IResult<Input<'a>, O, nom::error::VerboseError<Input<'a>>>;

    /// Parses an entry from the tree. Lines are of the form
    /// `<mode> <name>\0<oid>`
    pub(super) fn parse_tree_entry(i: Input) -> Result<(FileMode, &str, Digest)> {
        let (i, mode) = nom::bytes::complete::take_until(" ").parse(i)?;
        let (i, _) = nom::bytes::complete::tag(" ").parse(i)?;
        let (i, name) = nom::bytes::complete::take_until("\0").parse(i)?;
        let (i, _) = nom::bytes::complete::tag("\0").parse(i)?;
        let (i, oid) = nom::bytes::complete::take(20usize).parse(i)?;
        let oid = Digest(oid.try_into().unwrap());

        let mode = mode.to_str().unwrap().parse::<libc::mode_t>().unwrap();
        let mode = FileMode::from(mode);
        let name = name.to_str().unwrap();

        Ok((i, (mode, name, oid)))
    }
}

impl super::Tree {
    pub fn parse(mut bytes: &[u8], root: &Utf8Path, database: &Database) -> Result<Self> {
        let mut entries: BTreeMap<String, TreeEntry> = Default::default();

        while let Some(null_idx) = memchr::memchr(b'\0', bytes) {
            let (line, newbytes) = bytes.split_at(null_idx + 21);

            bytes = newbytes;

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
        let (_, (mode, name, oid)) = nom::parse_tree_entry(line)
            .map_err(|e| eyre!("Failed to parse tree entry: {:?}", e))?;

        let name = name.to_owned();

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
