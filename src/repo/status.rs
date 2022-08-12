use crate::blob::Blob;
use crate::index::IndexEntry;
use crate::storable::DatabaseObject;
use crate::tree::Tree;
use crate::Result;

use std::{collections::HashMap, fmt::Display};

use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use tap::Tap;

use super::Repo;

impl super::Repo {
    pub fn status(&self) -> Result<()> {
        let status = match Status::new(self)? {
            Some(x) => x,
            None => return Ok(()),
        };

        let statuses = status
            .get_statuses()?
            .tap_mut(|v| v.sort_unstable_by_key(|x| x.0));

        for (path, change) in statuses {
            println!("{} {}", change, path);
        }

        Ok(())
    }
}

pub struct Status<'r: 'i, 'i> {
    repo: &'r Repo,
    files: Vec<Utf8PathBuf>,
    index: HashMap<&'i Utf8Path, &'i IndexEntry>,
    head_tree: Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Untracked,
    Removed,
    Modified,
    IndexAdded,
    IndexRemoved,
    IndexModified,
}

impl Display for Change {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Change::Untracked => write!(f, "??"),
            Change::Modified => write!(f, " M"),
            Change::Removed => write!(f, " D"),
            Change::IndexAdded => write!(f, "A "),
            Change::IndexRemoved => write!(f, "D "),
            Change::IndexModified => write!(f, "M "),
        }
    }
}

impl<'r: 'i, 'i> Status<'r, 'i> {
    pub fn new(repo: &'r Repo) -> Result<Option<Self>> {
        let mut files = repo.list_files(Utf8Path::new("."))?;
        files.sort_unstable();

        let index = repo.index.entries();
        let index = index
            .iter()
            .map(|x| (x.path(), x))
            .collect::<HashMap<_, _>>();
        let tree = match Self::load_head_tree(repo)? {
            Some(tree) => tree,
            None => {
                eprintln!("No commits on this branch");
                return Ok(None);
            }
        };

        Ok(Some(Self {
            repo,
            files,
            index,
            head_tree: tree,
        }))
    }

    /// Loads the `Tree` of the head commit of the repo. Returns None if the repo does not have a
    /// HEAD.
    fn load_head_tree(repo: &Repo) -> Result<Option<Tree>> {
        match repo.read_head()? {
            Some(oid) => {
                let commit = repo
                    .database
                    .load(&oid)?
                    .into_commit()
                    .expect("HEAD should be a commit oid");
                Ok(repo.database.load(commit.tree_id())?.into_tree())
            }
            None => Ok(None),
        }
    }

    pub fn get_statuses(&self) -> Result<Vec<(&Utf8Path, Change)>> {
        let untracked = self.files.par_iter().filter_map(|path| {
            if !self.index.contains_key(&path.as_path()) {
                Some((path.as_path(), Change::Untracked))
            } else {
                None
            }
        });

        let mod_rem_add = self.index.par_iter().filter_map(|(&path, &entry)| {
            if self.is_modified(entry).unwrap() {
                Some((path, Change::Modified))
            } else if !self.repo.dir.join(path).exists() {
                Some((path, Change::Removed))
            } else if !self.head_tree.contains(entry.name()) {
                Some((path, Change::IndexAdded))
            } else if {
                let tree_entry = self.head_tree.get_entry(entry.name()).unwrap();
                tree_entry.oid() != entry.oid() || tree_entry.mode() != entry.mode()
            } {
                Some((path, Change::IndexModified))
            } else {
                None
            }
        });

        Ok(untracked.chain(mod_rem_add).collect())
    }

    /// Checks whether an index entry has been modified.
    ///
    /// Returns `true` if a file has been modified, `false` otherwise.
    fn is_modified(&self, entry: &IndexEntry) -> Result<bool> {
        let full_path = self.repo.dir.join(entry.path());
        let stat = match Repo::stat_file(&full_path)? {
            Some(x) => x,
            None => return Ok(false),
        };

        if !entry.stat_matches(&stat) {
            return Ok(true);
        }
        if entry.times_match(&stat) {
            return Ok(false);
        }
        let data = std::fs::read(full_path)?;
        let blob = Blob::new(data);
        let blob = DatabaseObject::new(&blob);
        let new_oid = blob.into_oid();

        Ok(*entry.oid() != new_oid)
    }
}
