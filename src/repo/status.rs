use crate::index::IndexEntry;
use crate::storable::DatabaseObject;
use crate::tree::Tree;
use crate::Result;
use crate::{blob::Blob, tree::TreeEntry};

use std::{collections::HashMap, fmt::Display};

use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use tap::Tap;

use super::Repo;

impl super::Repo {
    pub fn status(&self, long: bool) -> Result<()> {
        let status = match Status::new(self)? {
            Some(x) => x,
            None => return Ok(()),
        };

        let statuses = status
            .get_statuses()?
            .tap_mut(|v| v.sort_unstable_by_key(|x| x.0));

        if long {
            print_long_status(&statuses);
        } else {
            print_porcelain_status(&statuses);
        }

        Ok(())
    }
}

fn print_long_status(statuses: &[(&Utf8Path, Change)]) {
    let mut it = statuses.iter().filter(|x| x.1.is_index()).peekable();
    if it.peek().is_some() {
        println!("Changes to be committed:");
        for (path, status) in it {
            let word = match status {
                Change::IndexAdded => "new file",
                Change::IndexRemoved => "deleted",
                Change::IndexModified => "modified",
                _ => unreachable!(),
            };
            println!("{word}: {path}");
        }
    }
    // println!("  (use \"rit reset HEAD <file>...\" to unstage)");

    let mut it = statuses.iter().filter(|x| !x.1.is_index()).peekable();
    if it.peek().is_some() {
        println!("Changes not staged for commit:");
        for (path, status) in it {
            let word = match status {
                Change::Untracked => continue,
                Change::Removed => "deleted",
                Change::Modified => "modified",
                _ => unreachable!(),
            };
            println!("{word}: {path}");
        }
    }

    let mut it = statuses
        .iter()
        .filter(|x| matches!(x.1, Change::Untracked))
        .peekable();
    if it.peek().is_some() {
        println!("Untracked files:");
        for (path, _) in it {
            println!("{path}");
        }
    }
}

fn print_porcelain_status(statuses: &[(&Utf8Path, Change)]) {
    for (path, change) in statuses {
        println!("{} {}", change, path);
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

impl Change {
    fn is_index(self) -> bool {
        matches!(
            self,
            Change::IndexAdded | Change::IndexRemoved | Change::IndexModified
        )
    }
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

    #[allow(clippy::blocks_in_if_conditions)]
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

        let del = self.head_tree.iter().filter_map(|entry| {
            let path = Utf8Path::new(match entry {
                TreeEntry::File(f) => f.name(),
                TreeEntry::IncompleteFile { name, .. } => name,
                TreeEntry::Directory { name, .. } => name,
            });

            if !self.index.contains_key(path) {
                Some((path, Change::IndexRemoved))
            } else {
                None
            }
        });

        Ok(untracked
            .chain(mod_rem_add)
            .collect::<Vec<_>>()
            .tap_mut(|v| v.extend(del)))
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
