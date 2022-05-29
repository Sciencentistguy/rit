use std::{
    collections::HashMap,
    fs::Metadata,
    os::unix::prelude::*,
    path::{Path, PathBuf},
};

use color_eyre::eyre::eyre;
use color_eyre::Result;

use super::Storable;
use crate::digest::Digest;

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
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn filename(&self) -> Option<&str> {
        self.path().to_str()
    }

    fn parents(&self) -> Option<&Path> {
        self.path.parent()
    }
}

pub struct Tree {
    formatted: Vec<u8>,
    oid: Digest,
}

impl Tree {
    pub fn new(mut entries: Vec<Entry>) -> Self {
        entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));

        let mut data = Vec::new();
        for entry in &entries {
            let mode = format!("{:o}", entry.mode);
            data.extend_from_slice(mode.as_bytes());
            data.push(b' ');
            data.extend_from_slice(entry.path.as_os_str().as_bytes());
            data.push(b'\0');
            data.extend_from_slice(&*entry.oid);
        }

        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"tree ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(&data);
        let oid = Digest::new(&formatted);

        Self { formatted, oid }
    }

    pub fn build(entries: Vec<Entry>) -> Result<Self> {
        Ok(PartialTree::build(entries)?.freeze())
    }
}

#[derive(Debug)]
enum V {
    Entry(Entry),
    Tree(PartialTree),
}

#[derive(Debug)]
struct PartialTree {
    entries: HashMap<String, V>,
}

impl PartialTree {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn freeze(self) -> Tree {
        println!("freezing self: {:#?}", &self);
        todo!()
    }

    fn build(mut entries: Vec<Entry>) -> Result<PartialTree> {
        entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        let mut root = PartialTree {
            entries: HashMap::new(),
        };

        for entry in entries {
            println!("doing entry {:?}", entry.path());
            let mut parents = entry.parents();
            if parents == Some(Path::new("")) {
                parents = None;
            }
            root.add_entry(parents, &entry)?;
        }

        Ok(root)
    }

    // XXX this should not take parents and also entry, parents can be gotten from entry
    fn add_entry(&mut self, parents: Option<&Path>, entry: &Entry) -> Result<()> {
        match parents {
            None => {
                self.entries.insert(
                    entry.filename().unwrap().to_owned(),
                    V::Entry(entry.clone()),
                );
            }
            Some(parents) => {
                let basename = parents
                    .components()
                    .next()
                    .ok_or_else(|| eyre!("parents of '{:?}' was Some(empty)", entry.path()))?
                    .as_os_str()
                    .to_str()
                    .ok_or(eyre!("non-unicode path"))?
                    .to_owned();
                let tree = self
                    .entries
                    .entry(basename.clone())
                    .or_insert_with(|| V::Tree(PartialTree::new()));
                let tree = match tree {
                    V::Entry(_) => {
                        unreachable!("tree for '{:?}' should be tree not entry", entry.path())
                    }
                    V::Tree(x) => x,
                };

                let t = parents.strip_prefix(basename)?;

                tree.add_entry(if t == Path::new("") { None } else { Some(t) }, entry)?;
            }
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
