#![allow(dead_code)]

mod database;
mod digest;
mod interface;
mod lock;
mod refs;
mod storable;
mod util;
mod workspace;

pub use color_eyre::Result;
use walkdir::WalkDir;

use crate::database::Database;
use crate::digest::Digest;
use crate::interface::*;
use crate::refs::Refs;
use crate::storable::blob::Blob;
use crate::storable::commit::Author;
use crate::storable::commit::Commit;
use crate::storable::tree::Entry;
use crate::storable::tree::PartialTree;
use crate::storable::Storable;
use crate::workspace::Workspace;

use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use once_cell::sync::Lazy;
use tracing::*;
use tracing_subscriber::prelude::*;

static ARGS: Lazy<Opt> = Lazy::new(Opt::parse);

fn main() -> Result<()> {
    color_eyre::install().unwrap();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    Lazy::force(&ARGS);

    let repo = match ARGS.path {
        Some(ref path) => Repo::new(path.clone()),
        None => Repo::new(std::env::current_dir()?),
    };

    match &ARGS.command {
        Command::Init => repo.init()?,
        Command::Commit { message } => {
            let commit_id = repo.commit(
                message
                    .as_deref()
                    .expect("Using an editor for commit message is currently unimplemented"),
            )?;
            println!("Created commit {}", commit_id.to_hex())
        }
    }
    Ok(())
}

struct Repo {
    dir: PathBuf,
    workspace: Workspace,
    refs: Refs,
    database: Database,
}

impl Repo {
    fn new(repo_root: PathBuf) -> Self {
        let workspace = Workspace::new(&repo_root);
        let refs = Refs::new(&repo_root);
        let database = Database::new(&repo_root);
        trace!(path=?repo_root, "Opened repo");
        Self {
            dir: repo_root,
            workspace,
            refs,
            database,
        }
    }

    fn init(&self) -> Result<()> {
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

    fn commit(&self, message: &str) -> Result<Digest> {
        trace!(path=?self.dir, %message, "Starting commit");
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

        let mut root = PartialTree::build(entries)?;
        trace!("Traversing root");
        root.traverse(|tree| self.database.store(&tree.freeze()))?;

        let root = root.freeze();

        self.database.store(&root)?;

        let parent_commit = self.refs.read_head()?;

        let author = Author {
            name: std::env::var("RIT_AUTHOR_NAME")?,
            email: std::env::var("RIT_AUTHOR_EMAIL")?,
        };

        let commit = Commit::new(parent_commit, root.into_oid(), author, message);
        self.database.store(&commit)?;
        self.refs.set_head(commit.get_oid())?;

        Ok(commit.into_oid())
    }
}

#[cfg(test)]
mod test;
