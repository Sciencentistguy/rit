use crate::blob::Blob;
use crate::index::IndexEntry;
use crate::storable::DatabaseObject;
use crate::tree::Tree;
use crate::Result;

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;

impl super::Repo {
    pub fn status(&self) -> Result<()> {
        let (files, index) = self.get_files_and_index()?;
        let tree = match self.load_head_tree()? {
            Some(tree) => tree,
            None => {
                eprintln!("No commits on this branch");
                return Ok(());
            }
        };

        // let tree = self.load_head_tree().ok_or_else(|| eyre!("Cannot call status on"))

        self.untracked_files(&files, &index).for_each(|path| {
            println!("?? {}", path);
        });

        self.changed_files(&index).for_each(|path| {
            println!(" M {}", path);
        });

        self.deleted_files(&index).for_each(|path| {
            println!(" D {}", path);
        });

        self.added_files(&index, &tree).for_each(|path| {
            println!("A  {}", path);
        });

        Ok(())
    }

    /// Checks whether an index entry has been modified.
    /// Returns `true` if a file has been modified, `false` otherwise.
    fn check_index_entry(&self, entry: &IndexEntry) -> Result<bool> {
        let full_path = self.dir.join(entry.path());
        let stat = match Self::stat_file(&full_path)? {
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

    /// Returns an iterator over the untracked files in the repo
    pub fn untracked_files<'a>(
        &self,
        files: &'a [Utf8PathBuf],
        index: &'a HashMap<&Utf8Path, &IndexEntry>,
    ) -> impl ParallelIterator<Item = &'a Utf8Path> {
        files
            .par_iter()
            .filter(|path| !index.contains_key(&path.as_path()))
            .map(|x| x.as_path())
    }

    /// Returns an iterator over the changed files in the repo
    pub fn changed_files<'a: 's, 's>(
        &'s self,
        index: &'a HashMap<&Utf8Path, &IndexEntry>,
    ) -> impl ParallelIterator<Item = &'a Utf8Path> + 's {
        index
            .par_iter()
            .filter(|(_, &entry)| self.check_index_entry(entry).unwrap())
            .map(|(p, _)| *p)
    }

    /// Returns an iterator over the deleted files in the repo
    pub fn deleted_files<'a: 's, 's>(
        &'s self,
        index: &'a HashMap<&Utf8Path, &IndexEntry>,
    ) -> impl ParallelIterator<Item = &'a Utf8Path> + 's {
        index
            .par_iter()
            .filter(|(&p, _)| !self.dir.join(p).exists())
            .map(|(&p, _)| p)
    }

    /// Returns a tuple of: A `Vec` of all files in the Repo and a `HashMap` of all index entries.
    pub fn get_files_and_index(
        &self,
    ) -> Result<(Vec<Utf8PathBuf>, HashMap<&Utf8Path, &IndexEntry>)> {
        let mut files = self.list_files(Utf8Path::new("."))?;
        files.sort_unstable();

        let index = self.index.entries();
        let index = index
            .iter()
            .map(|x| (x.path(), x))
            .collect::<HashMap<_, _>>();
        Ok((files, index))
    }

    /// Loads the `Tree` of the head commit of the repo. Returns None if the repo does not have a
    /// HEAD.
    fn load_head_tree(&self) -> Result<Option<Tree>> {
        match self.read_head()? {
            Some(oid) => {
                let commit = self
                    .database
                    .load(&oid)?
                    .into_commit()
                    .expect("HEAD should be a commit oid");
                Ok(self.database.load(commit.tree_id())?.into_tree())
            }
            None => Ok(None),
        }
    }

    /// Returns an iterator over the added files in the repo.
    pub fn added_files<'a: 's, 's>(
        &'s self,
        index: &'a HashMap<&Utf8Path, &IndexEntry>,
        last_tree: &'a Tree,
    ) -> impl ParallelIterator<Item = &'a Utf8Path> + 's {
        index
            .par_iter()
            .filter(|(_, &entry)| {
                let name = entry.name();
                // dbg!(&name);
                !last_tree.contains(name)
            })
            .map(|(p, _)| *p)
    }
}
