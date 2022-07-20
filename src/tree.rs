use std::{collections::BTreeMap, path::Path};

use color_eyre::Result;
use once_cell::sync::OnceCell;
use tracing::*;

use crate::storable::Storable;
use crate::{filemode::FileMode, repo::index::IndexEntry, Digest};

#[derive(Debug)]
enum TreeEntry {
    File(IndexEntry),
    Directory(Tree),
}

impl TreeEntry {
    fn mode(&self) -> FileMode {
        match self {
            TreeEntry::File(f) => f.mode(),
            TreeEntry::Directory(_) => FileMode::DIRECTORY,
        }
    }
}

#[derive(Debug)]
pub struct Tree {
    entries: BTreeMap<String, TreeEntry>,
    oid: OnceCell<Digest>,
}

impl Tree {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            oid: OnceCell::new(),
        }
    }

    pub fn build(entries: &[IndexEntry]) -> Result<Tree> {
        let mut root = Tree::new();

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
            if let TreeEntry::Directory(entry) = entry {
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
                .insert(filename.to_owned(), TreeEntry::File(entry.clone()));
        } else {
            let tree = Tree::new();
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
                .or_insert(TreeEntry::Directory(tree));
            let tree = match tree {
                TreeEntry::Directory(tree) => tree,
                _ => unreachable!("entry should be a tree"),
            };

            trace!(?parents, "Recursing...");
            tree.add_entry(&parents[1..], entry)?;
        }
        Ok(())
    }
}

impl Storable for Tree {
    fn format(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for (name, entry) in self.entries.iter() {
            data.extend_from_slice(format!("{:o}", entry.mode()).as_bytes());
            data.push(b' ');
            data.extend_from_slice(name.as_bytes());
            data.push(b'\0');
            let oid = match entry {
                TreeEntry::File(f) => f.digest(),
                TreeEntry::Directory(d) => {
                    d.oid.get().expect("subtree oid should have been inited")
                }
            };
            data.extend_from_slice(&**oid);
        }

        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"tree ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(&data);

        // XXX: hashing twice

        let oid = Digest::new(&formatted);

        match self.oid.set(oid.clone()) {
            Ok(_) => {}
            Err(e) => {
                assert!(oid == e);
            }
        }

        formatted
    }
}
