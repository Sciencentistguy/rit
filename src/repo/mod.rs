mod database;
mod index;
mod refs;
mod workspace;

use database::Database;
// use index::Index;

use crate::{
    digest::Digest,
    storable::{blob::Blob, commit::Author, commit::Commit, tree::PartialTree, Storable},
    Result,
};

use std::path::PathBuf;

use tracing::*;

use self::index::IndexWrapper;

pub struct Repo {
    dir: PathBuf,
    head_path: PathBuf,
    database: Database,
    index: IndexWrapper,
}

impl Repo {
    pub fn open(repo_root: PathBuf) -> Self {
        trace!(path=?repo_root, "Opening repo");
        let database = Database::new(&repo_root);
        let index = IndexWrapper::open(&repo_root);
        let head_path = repo_root.join(".git/HEAD");
        Self {
            dir: repo_root,
            head_path,
            database,
            index,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        trace!(path=?self.dir, "Initialising repo");
        let git_dir = self.dir.join(".git");
        if git_dir.exists() {
            warn!("Repo already exists, init will do nothing");
        } else {
            for d in ["objects", "refs"] {
                let dir = git_dir.join(d);
                trace!(path=?dir, "Creating directory");
                std::fs::create_dir_all(dir)?;
            }
        }
        Ok(())
    }

    pub fn commit(&mut self, message: &str) -> Result<Digest> {
        trace!(path=?self.dir, %message, "Starting commit");
        let entries = self.create_entries()?;
        let mut root = PartialTree::build(entries)?;
        trace!("Traversing root");
        root.traverse(|tree| self.database.store(&tree.freeze()))?;

        let root = root.freeze();

        self.database.store(&root)?;

        let parent_commit = self.read_head()?;

        let author = Author {
            name: std::env::var("RIT_AUTHOR_NAME")?,
            email: std::env::var("RIT_AUTHOR_EMAIL")?,
        };

        let commit = Commit::new(parent_commit, root.into_oid(), author, message);
        self.database.store(&commit)?;
        self.set_head(commit.get_oid())?;

        Ok(commit.into_oid())
    }

    pub fn add(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            let paths = self.list_files(path)?;
            for path in paths {
                trace!(?path, "Adding file");
                let abs_path = self.dir.join(&path);

                let data = std::fs::read(&abs_path)?;
                let stat = Self::stat_file(&abs_path);

                let blob = Blob::new(&data);
                self.database.store(&blob)?;
                self.index.add(&path, blob.get_oid(), stat);
            }
        }
        self.index.write_out()?;

        Ok(())
    }
}
