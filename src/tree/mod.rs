mod parse;
mod write;

use std::collections::BTreeMap;

use camino::Utf8Path;
use color_eyre::Result;
use once_cell::sync::OnceCell;
use tracing::*;

use crate::{filemode::FileMode, index::IndexEntry, util::Descends, Digest};

#[derive(Debug)]
pub enum TreeEntry {
    File(IndexEntry),
    IncompleteFile {
        oid: Digest,
        name: String,
        mode: FileMode,
    },
    Directory {
        tree: Tree,
        name: String,
    },
}

impl TreeEntry {
    pub fn mode(&self) -> FileMode {
        match self {
            TreeEntry::File(f) => f.mode(),
            TreeEntry::Directory { .. } => FileMode::DIRECTORY,
            TreeEntry::IncompleteFile { mode, .. } => *mode,
        }
    }

    pub fn oid(&self) -> Option<&Digest> {
        match self {
            TreeEntry::File(f) => Some(f.oid()),
            TreeEntry::Directory { tree, .. } => tree.oid.get(),
            TreeEntry::IncompleteFile { oid, .. } => Some(oid),
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
            if let TreeEntry::Directory { tree: entry, .. } = entry {
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
            let name = parents[0]
                .file_name()
                .expect("Entry should have a file name")
                .to_owned();
            let tree = self
                .entries
                .entry(name.clone())
                .or_insert(TreeEntry::Directory { tree, name });
            let tree = match tree {
                TreeEntry::Directory { tree, .. } => tree,
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

    pub fn contains(&self, name: &str) -> bool {
        if self.entries.contains_key(name) {
            return true;
        }
        for entry in self.entries.values() {
            if let TreeEntry::Directory { tree, .. } = entry {
                let name = Utf8Path::new(name);
                let name = name.file_name().unwrap();
                if tree.contains(name) {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_entry(&self, name: &str) -> Option<&IndexEntry> {
        let path = Utf8Path::new(name);
        if let Some(entry) = self.entries.get(name) {
            if let TreeEntry::File(entry) = entry {
                Some(entry)
            } else {
                None
            }
        } else {
            let top_of_path = path.descends()[0];
            if let Some(TreeEntry::Directory { tree, .. }) = self.entries.get(top_of_path.as_str())
            {
                let rest = path.strip_prefix(top_of_path).unwrap();
                tree.get_entry(rest.as_str())
            } else {
                None
            }
        }
    }

    pub fn oid(&self) -> Option<&Digest> {
        self.oid.get()
    }
}
