use std::{
    collections::BTreeMap,
    fs::Metadata,
    io::Write,
    os::unix::prelude::*,
    path::{Path, PathBuf},
};

use color_eyre::Result;
use tracing::*;

use super::Storable;
use crate::{util::Descends, Digest};

#[derive(Clone, Copy)]
pub struct Mode(u32);

impl std::fmt::Octal for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:o}", self.0)
    }
}

#[derive(Clone)]
pub struct Entry {
    path: PathBuf,
    oid: Digest,
    mode: Mode,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry").field("path", &self.path).finish()
    }
}

impl Entry {
    pub fn new(filename: PathBuf, oid: Digest, metadata: Metadata) -> Self {
        Self {
            path: filename,
            oid,
            //FIXME: unix-specific
            mode: Mode(metadata.mode()),
        }
    }

    fn parents(&self) -> Vec<&Path> {
        let mut v = self.path.descends();
        v.pop();
        v
    }
}

pub struct Tree {
    formatted: Vec<u8>,
    oid: Digest,
}

// impl Tree {
// pub fn new(mut entries: Vec<Entry>) -> Self {
// entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));

// let mut data = Vec::new();
// for entry in &entries {
// let mode = format!("{:o}", entry.mode);
// data.extend_from_slice(mode.as_bytes());
// data.push(b' ');
// data.extend_from_slice(entry.path.as_os_str().as_bytes());
// data.push(b'\0');
// data.extend_from_slice(&*entry.oid);
// }

// let mut formatted = Vec::new();
// formatted.extend_from_slice(b"tree ");
// formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
// formatted.push(b'\0');
// formatted.extend_from_slice(&data);
// let oid = Digest::new(&formatted);

// Self { formatted, oid }
// }

// pub fn build(entries: Vec<Entry>) -> Result<Self> {
// trace!("Building tree of entries");
// let pt = PartialTree::build(entries)?;
// trace!(tree = ?pt, "Finished building tree");
// let t = pt.freeze();
// todo!();
// // Ok()
// }
// }

#[derive(Debug)]
enum PartialTreeEntry {
    File(Entry),
    Directory(PartialTree),
}

impl PartialTreeEntry {
    fn mode(&self) -> Mode {
        match self {
            PartialTreeEntry::File(f) => f.mode,
            PartialTreeEntry::Directory(_) => DIRECTORY_MODE,
        }
    }
}

#[derive(Debug)]
pub struct PartialTree {
    entries: BTreeMap<String, PartialTreeEntry>,
    oid: Option<Digest>,
}

const DIRECTORY_MODE: Mode = Mode(0o040000);

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
                PartialTreeEntry::File(f) => &f.oid,
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

    pub fn build(mut entries: Vec<Entry>) -> Result<PartialTree> {
        entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        let mut root = PartialTree::new();

        for entry in entries {
            trace!(?entry, "Inserting entry into tree");
            let parents = entry.parents();
            trace!(?parents, "Parents of entry");
            root.add_entry(&parents, &entry)?;
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

    fn add_entry(&mut self, parents: &[&'_ Path], entry: &Entry) -> Result<()> {
        if parents.is_empty() {
            let filename = entry
                .path
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
