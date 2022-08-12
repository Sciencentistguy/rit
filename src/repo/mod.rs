mod add;
mod commit;
pub mod database;
mod head;
mod ignore;
mod refs;
mod show_head;
pub mod status;
mod workspace;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use tracing::*;

use crate::index::IndexWrapper;
use crate::Result;

use self::database::Database;

pub struct Repo {
    pub dir: Utf8PathBuf,
    pub git_dir: Utf8PathBuf,
    pub head_path: Utf8PathBuf,
    pub database: Database,
    pub index: IndexWrapper,
}

impl Repo {
    pub fn open(repo_root: Utf8PathBuf) -> Result<Self> {
        let git_dir = repo_root.join(".git");
        if !git_dir.exists() {
            return Err(eyre!(
                "Failed to open repository: directory is not a git repository: '{}'",
                repo_root
            ));
        }

        trace!(path=?repo_root, "Opening repo");
        let database = Database::new(&git_dir);
        let index = IndexWrapper::open(&git_dir);
        let head_path = git_dir.join("HEAD");
        Ok(Self {
            dir: repo_root,
            git_dir,
            head_path,
            database,
            index,
        })
    }

    pub fn init(path: &Utf8Path) -> Result<()> {
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
}
