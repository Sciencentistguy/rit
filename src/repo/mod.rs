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

use std::path::Path;
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
    pub fn new(repo_root: PathBuf) -> Self {
        let database = Database::new(&repo_root);
        let index = IndexWrapper::open(&repo_root);
        trace!(path=?repo_root, "Opened repo");
        let head_path = repo_root.join(".git/HEAD");
        Self {
            dir: repo_root,
            head_path,
            database,
            index,
        }
    }

    pub fn init(&self) -> Result<()> {
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

    pub fn commit(&self, message: &str) -> Result<Digest> {
        trace!(path=?self.dir, %message, "Starting commit");
        let entries = self.list_files()?;
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

    pub fn add(&self, path: &Path) -> Result<()> {
        trace!(?path, "Adding file");

        let data = std::fs::read(path)?;
        let stat = Self::stat_file(path);
        // let index = todo!();

        // let blob = Blob::new(&data);
        // self.database.store(&blob)?;
        // index.add(path, blob.oid, stat);
        // index.write_updates();

        todo!()
    }
}
