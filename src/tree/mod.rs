mod parse;
mod write;

use std::collections::BTreeMap;

use camino::Utf8Path;
use color_eyre::Result;
use once_cell::sync::OnceCell;
use tracing::*;

use crate::{filemode::FileMode, index::IndexEntry, storable::Storable, util::Descends, Digest};

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
            TreeEntry::Directory { .. } => FileMode::Directory,
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

    pub fn as_file(&self) -> Option<&IndexEntry> {
        if let Self::File(v) = self {
            Some(v)
        } else {
            None
        }
    }

    const fn kind(&self) -> &'static str {
        match self {
            TreeEntry::File(_) | TreeEntry::IncompleteFile { .. } => "blob",
            TreeEntry::Directory { .. } => "tree",
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

    pub fn contains(&self, name: impl AsRef<str>) -> bool {
        let name = name.as_ref();
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

    pub fn iter(&self) -> impl Iterator<Item = &TreeEntry> + '_ {
        struct Iter<'a> {
            tree: &'a Tree,
            stack: Vec<Box<dyn Iterator<Item = &'a TreeEntry> + 'a>>,
        }

        impl<'a> Iter<'a> {
            fn new(tree: &'a Tree) -> Self {
                let it = tree.entries.iter().map(|x| x.1);
                Self {
                    tree,
                    stack: vec![Box::new(it)],
                }
            }
        }

        impl<'a> Iterator for Iter<'a> {
            type Item = &'a TreeEntry;

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(x) = self.stack.first_mut() {
                    let next = x.next();
                    match next {
                        Some(ent @ TreeEntry::File(_)) => Some(ent),

                        Some(ent @ TreeEntry::IncompleteFile { .. }) => Some(ent),

                        Some(TreeEntry::Directory { tree, .. }) => {
                            let it = tree.entries.iter().map(|x| x.1);
                            self.stack.push(Box::new(it));
                            self.next()
                        }

                        None => {
                            let _ = self.stack.pop();
                            if self.stack.is_empty() {
                                return None;
                            }
                            self.next()
                        }
                    }
                } else {
                    unreachable!();
                }
            }
        }

        Iter::new(self)
    }

    pub fn pretty_print(&self) -> std::io::Result<()> {
        for (name, entry) in &self.entries {
            if let TreeEntry::Directory { tree, .. } = entry {
                let _ = tree.format(); // force the tree to caluclate all its oids
            }
            println!(
                "{} {} {:x}\t{name}",
                entry.mode(),
                entry.kind(),
                entry
                    .oid()
                    .expect("tree loaded from disk should have oid set")
            );
        }

        Ok(())
    }
}
