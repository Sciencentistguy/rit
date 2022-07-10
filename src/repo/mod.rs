mod database;
mod refs;

use walkdir::WalkDir;

use database::Database;

use crate::digest::Digest;
use crate::storable::blob::Blob;
use crate::storable::commit::Author;
use crate::storable::commit::Commit;
use crate::storable::tree::Entry;
use crate::storable::tree::PartialTree;
use crate::storable::Storable;
use crate::Result;

use std::path::Path;
use std::path::PathBuf;

use tracing::*;

pub struct Repo {
    dir: PathBuf,
    head_path: PathBuf,
    database: Database,
}

impl Repo {
    pub fn new(repo_root: PathBuf) -> Self {
        let database = Database::new(&repo_root);
        trace!(path=?repo_root, "Opened repo");
        let head_path = repo_root.join(".git/HEAD");
        Self {
            dir: repo_root,
            head_path,
            database,
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
}

// Workspace
impl Repo {
    fn list_files(&self) -> Result<Vec<Entry>> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(&self.dir) {
            let entry = entry?;
            let path = entry.path();
            if path
                .components()
                .any(|c| AsRef::<Path>::as_ref(&c) == Path::new(".git"))
            {
                continue;
            }
            trace!(?path, "Found entry");
            if !path.is_dir() {
                let data = std::fs::read(&path)?;
                let blob = Blob::new(&data);
                self.database.store(&blob)?;
                let metadata = std::fs::metadata(&path)?;
                entries.push(Entry::new(
                    path.strip_prefix(&self.dir)?.to_owned(),
                    blob.into_oid(),
                    metadata,
                ));
            }
        }
        Ok(entries)
    }
}
