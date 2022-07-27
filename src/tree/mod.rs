mod parse;
mod write;

use std::collections::BTreeMap;

use camino::Utf8Path;
use color_eyre::Result;
use once_cell::sync::OnceCell;
use tracing::*;

use crate::{filemode::FileMode, index::IndexEntry, Digest};

#[derive(Debug)]
pub enum TreeEntry {
    File(IndexEntry),
    Directory(Tree),
    Database {
        oid: Digest,
        name: String,
        mode: FileMode,
    },
}

impl TreeEntry {
    fn mode(&self) -> FileMode {
        match self {
            TreeEntry::File(f) => f.mode(),
            TreeEntry::Directory(_) => FileMode::DIRECTORY,
            TreeEntry::Database {
                oid: _,
                name: _,
                mode,
            } => *mode,
        }
    }

    pub fn oid(&self) -> Option<&Digest> {
        match self {
            TreeEntry::File(f) => Some(f.oid()),
            TreeEntry::Directory(t) => t.oid.get(),
            TreeEntry::Database { oid, .. } => Some(oid),
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
            trace!(entry = entry.name(), "Inserting entry into tree");
            let parents = entry.parents();
            trace!(?parents, "Parents of entry");
            root.add_entry(&parents, entry)?;
        }

        Ok(root)
    }

    pub fn traverse<F>(&self, f: F) -> Result<()>
    where
        F: Fn(&Self) -> Result<()> + Copy,
    {
        for (name, entry) in self.entries.iter() {
            if let TreeEntry::Directory(entry) = entry {
                trace!(%name, "Traversing subtree");
                entry.traverse(f)?;
            }
        }
        f(self)
    }

    fn add_entry(&mut self, parents: &[&'_ Utf8Path], entry: &IndexEntry) -> Result<()> {
        if parents.is_empty() {
            let filename = entry
                .path()
                .file_name()
                .expect("Entry with no parents must have a filename");
            self.entries
                .insert(filename.to_owned(), TreeEntry::File(entry.clone()));
        } else {
            let tree = Tree::new();
            let tree = self
                .entries
                .entry(
                    parents[0]
                        .file_name()
                        .expect("Entry should have a file name")
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

    pub fn entries(&self) -> &BTreeMap<String, TreeEntry> {
        &self.entries
    }
}
