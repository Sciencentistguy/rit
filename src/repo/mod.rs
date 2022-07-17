mod database;
pub mod index;
mod refs;
mod workspace;

use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;
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
    pub dir: PathBuf,
    pub head_path: PathBuf,
    pub database: Database,
    pub index: IndexWrapper,
}

impl Repo {
    pub fn open(repo_root: PathBuf) -> Result<Self> {
        let git_dir = repo_root.join(".git");
        if !git_dir.exists() {
            return Err(eyre!(
                "Failed to open repository: directory is not a git repository: '{}'",
                repo_root.display()
            ));
        }

        trace!(path=?repo_root, "Opening repo");
        let database = Database::new(&git_dir);
        let index = IndexWrapper::open(&git_dir);
        let head_path = git_dir.join("HEAD");
        Ok(Self {
            dir: repo_root,
            head_path,
            database,
            index,
        })
    }

    pub fn init(path: &Path) -> Result<()> {
        trace!(?path, "Initialising repo");
        let git_dir = path.join(".git");
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
        let entries = &self.index.entries();
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
        self.set_head(commit.oid())?;

        Ok(commit.into_oid())
    }

    pub fn add(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            trace!(?path, "Adding file to repo");
            if !self.dir.join(path).exists() {
                return Err(eyre!("Path does not exist: {}", path.display()));
            }
            let paths = self.list_files(path)?;
            for path in paths {
                let path = if path.has_root() {
                    path.strip_prefix(&self.dir)
                        .wrap_err(format!("Path: {:?}", path))?
                } else {
                    &path
                };
                trace!(?path, "Adding file");
                let abs_path = self.dir.join(&path);

                let data = std::fs::read(&abs_path)
                    .wrap_err(format!("Failed to read file: {}", abs_path.display()))?;
                let stat = Self::stat_file(&abs_path);

                let blob = Blob::new(&data);
                self.database.store(&blob)?;
                self.index.add(path, blob.oid(), stat);
            }
        }
        self.index.write_out()?;

        Ok(())
    }
}
