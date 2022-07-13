use std::{
    collections::BTreeMap,
    fs::Metadata,
    os::unix::prelude::*,
    path::{Path, PathBuf},
};

use color_eyre::Result;
use tracing::*;

use super::Storable;
use crate::{filemode::FileMode, repo::index::IndexEntry, util::Descends, Digest};

#[derive(Clone)]
pub struct Entry {
    path: PathBuf,
    oid: Digest,
    mode: FileMode,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry").field("path", &self.path).finish()
    }
}

impl Entry {
    pub fn new(filename: PathBuf, oid: Digest, metadata: Metadata) -> Self {
        let mode = if FileMode(metadata.mode()).is_executable() {
            FileMode::EXECUTABLE
        } else {
            FileMode::REGULAR
        };

        Self {
            path: filename,
            oid,
            mode,
        }
    }
}

pub struct Tree {
    formatted: Vec<u8>,
    oid: Digest,
}

#[derive(Debug)]
enum PartialTreeEntry {
    File(IndexEntry),
    Directory(PartialTree),
}

impl PartialTreeEntry {
    fn mode(&self) -> FileMode {
        match self {
            PartialTreeEntry::File(f) => f.mode(),
            PartialTreeEntry::Directory(_) => FileMode::DIRECTORY,
        }
    }
}

#[derive(Debug)]
pub struct PartialTree {
    entries: BTreeMap<String, PartialTreeEntry>,
    oid: Option<Digest>,
}

impl PartialTree {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            oid: None,
        }
    }

    pub fn freeze(&mut self) -> Tree {
        let mut data = Vec::new();
        for (name, entry) in self.entries.iter() {
            data.extend_from_slice(format!("{:o}", entry.mode()).as_bytes());
            data.push(b' ');
            data.extend_from_slice(name.as_bytes());
            data.push(b'\0');
            let oid = match entry {
                PartialTreeEntry::File(f) => &f.oid(),
                PartialTreeEntry::Directory(d) => {
                    d.oid.as_ref().expect("subtree oid should have been inited")
                }
            };
            data.extend_from_slice(&**oid);
        }
        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"tree ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(&data);
        let oid = Digest::new(&formatted);
        self.oid = Some(oid.clone());

        Tree { formatted, oid }
    }

    pub fn build(entries: &[IndexEntry]) -> Result<PartialTree> {
        let mut root = PartialTree::new();

        for entry in entries {
            trace!(entry=?std::str::from_utf8(entry.name()), "Inserting entry into tree");
            let parents = entry.parents();
            trace!(?parents, "Parents of entry");
            root.add_entry(&parents, entry)?;
        }

        Ok(root)
    }

    pub fn traverse<F>(&mut self, f: F) -> Result<()>
    where
        F: Fn(&mut Self) -> Result<()> + Copy,
    {
        for (name, entry) in self.entries.iter_mut() {
            if let PartialTreeEntry::Directory(entry) = entry {
                trace!(%name, "Traversing subtree");
                entry.traverse(f)?;
            }
        }
        f(self)
    }

    fn add_entry(&mut self, parents: &[&'_ Path], entry: &IndexEntry) -> Result<()> {
        if parents.is_empty() {
            let filename = entry
                .path()
                .file_name()
                .expect("Entry with no parents must have a filename")
                .to_str()
                .expect("file name should be utf-8");
            let filename = filename;
            self.entries
                .insert(filename.to_owned(), PartialTreeEntry::File(entry.clone()));
        } else {
            let tree = PartialTree::new();
            let tree = self
                .entries
                .entry(
                    parents[0]
                        .file_name()
                        .expect("should have a file name")
                        .to_str()
                        .expect("file name should be utf-8")
                        .to_owned(),
                )
                .or_insert(PartialTreeEntry::Directory(tree));
            let tree = match tree {
                PartialTreeEntry::Directory(tree) => tree,
                _ => unreachable!("entry should be a tree"),
            };

            trace!(?parents, "Recursing...");
            tree.add_entry(&parents[1..], entry)?;
        }
        Ok(())
    }
}

impl Storable for Tree {
    fn formatted(&self) -> &[u8] {
        // "{type} {len}\0{formatted}"
        &self.formatted
    }

    fn get_oid(&self) -> &Digest {
        &self.oid
    }

    fn into_oid(self) -> Digest {
        self.oid
    }
}
