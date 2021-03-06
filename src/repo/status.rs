use crate::storable::blob::Blob;
use crate::storable::tree::TreeEntry;
use crate::storable::Storable;
use crate::Result;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use super::index::IndexEntry;

impl super::Repo {
    pub fn status(&self) -> Result<()> {
        let (files, index) = self.read_status()?;

        self.untracked_files(&files, &index).for_each(|path| {
            println!("?? {}", path.display());
        });

        self.changed_files(&index).for_each(|path| {
            println!(" M {}", path.display());
        });

        self.deleted_files(&index).for_each(|path| {
            println!(" D {}", path.display());
        });

        Ok(())
    }

    fn check_index_entry(&self, entry: &IndexEntry) -> Result<bool> {
        let full_path = self.dir.join(entry.path());
        let stat = Self::stat_file(&full_path);
        if !entry.stat_matches(&stat) {
            return Ok(true);
        }
        if entry.times_match(&stat) {
            return Ok(false);
        }
        let data = std::fs::read(full_path)?;
        let new_oid = Blob::new(&data).into_oid();
        Ok(*entry.oid() != new_oid)
    }

    pub fn untracked_files<'a>(
        &self,
        files: &'a [PathBuf],
        index: &'a HashMap<&Path, &IndexEntry>,
    ) -> impl ParallelIterator<Item = &'a Path> {
        files
            .par_iter()
            .filter(|path| !index.contains_key(&path.as_path()))
            .map(|x| x.as_path())
    }

    pub fn changed_files<'a: 's, 's>(
        &'s self,
        index: &'a HashMap<&Path, &IndexEntry>,
    ) -> impl ParallelIterator<Item = &'a Path> + 's {
        index
            .par_iter()
            .filter(|(_, &entry)| {
                self.check_index_entry(entry)
                    .expect("Failed to check index entry")
            })
            .map(|(p, _)| *p)
    }

    pub fn deleted_files<'a: 's, 's>(
        &'s self,
        index: &'a HashMap<&Path, &IndexEntry>,
    ) -> impl ParallelIterator<Item = &'a Path> + 's {
        index
            .par_iter()
            .filter(|(&p, _)| !self.dir.join(p).exists())
            .map(|(&p, _)| p)
    }

    pub fn read_status(&self) -> Result<(Vec<PathBuf>, HashMap<&Path, &IndexEntry>)> {
        let mut files = self.list_files(Path::new("."))?;
        files.sort_unstable();

        let index = self.index.entries();
        let index = index
            .iter()
            .map(|x| (x.path(), x))
            .collect::<HashMap<_, _>>();
        Ok((files, index))
    }
}
