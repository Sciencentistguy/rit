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
use crate::storable::tree::Tree;
use crate::storable::Storable;
use crate::workspace::Workspace;

use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use once_cell::sync::Lazy;
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
        Self {
            dir: repo_root,
            workspace,
            refs,
            database,
        }
    }

    fn init(&self) -> Result<()> {
        let dir = self.dir.join(".git");
        for d in ["objects", "refs"] {
            std::fs::create_dir_all(dir.join(d))?;
        }
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<Digest> {
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
            if !path.is_dir() {
                println!("file: {:?}", path.canonicalize());
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

        for entry in &entries {
            println!("{:?}", entry.path());
        }

        let tree = Tree::build(entries)?;
        self.database.store(&tree)?;
        dbg!(tree.get_oid());

        let parent_commit = self.refs.read_head()?;

        let author = Author {
            name: std::env::var("RIT_AUTHOR_NAME")?,
            email: std::env::var("RIT_AUTHOR_EMAIL")?,
        };

        let commit = Commit::new(parent_commit, tree.into_oid(), author, message);
        self.database.store(&commit)?;
        self.refs.set_head(commit.get_oid())?;

        Ok(commit.into_oid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::Permissions;
    use std::io::{self, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};
    use tempdir::TempDir;

    #[test]
    fn rit_init() -> Result<()> {
        let dir = TempDir::new("")?;
        let repo = Repo::new(dir.path().to_owned());
        repo.init()?;

        let git_dir = dir.as_ref().join(".git");
        let obj_dir = git_dir.join("objects");
        let refs_dir = git_dir.join("refs");

        assert!(git_dir.is_dir());
        assert!(obj_dir.is_dir());
        assert!(refs_dir.is_dir());

        Ok(())
    }

    #[test]
    fn rit_commit() -> Result<()> {
        fn write_test_files(path: &Path) -> io::Result<()> {
            writeln!(std::fs::File::create(path.join("file1"))?, "hello")?;
            writeln!(std::fs::File::create(path.join("file2"))?, "world")?;
            std::fs::set_permissions(path.join("file2"), Permissions::from_mode(0o100755))?;
            Ok(())
        }
        let dir_rit = TempDir::new("")?;
        let dir_rit = dir_rit.path();
        let dir_git = TempDir::new("")?;
        let dir_git = dir_git.path();

        const COMMIT_NAME: &str = "Jamie Quigley";
        const COMMIT_EMAIL: &str = "jamie@quigley.xyz";

        std::env::set_var("RIT_AUTHOR_NAME", COMMIT_NAME);
        std::env::set_var("RIT_AUTHOR_EMAIL", COMMIT_EMAIL);

        let git_command_args = [
            "-c",
            &format!("user.name={}", COMMIT_NAME),
            "-c",
            &format!("user.email={}", COMMIT_EMAIL),
            "-c",
            "commit.gpgsign=false",
        ];

        let rit_repo = Repo::new(dir_rit.to_owned());

        // Rit create files
        rit_repo.init()?;
        // create test files
        write_test_files(dir_rit)?;
        let commit_id = rit_repo.commit("test")?;

        Command::new("git")
            .args(&git_command_args)
            .arg("init")
            .current_dir(&dir_git)
            .stdout(Stdio::null())
            .status()
            .unwrap();

        writeln!(std::fs::File::create(dir_git.join("file1"))?, "hello")?;
        writeln!(std::fs::File::create(dir_git.join("file2"))?, "world")?;
        std::fs::set_permissions(dir_git.join("file2"), Permissions::from_mode(0o100755))?;

        Command::new("git")
            .args(&git_command_args)
            .arg("add")
            .arg("--all")
            .current_dir(&dir_git)
            .stdout(Stdio::null())
            .status()?;
        Command::new("git")
            .args(&git_command_args)
            .arg("commit")
            .arg("-m")
            .arg("test")
            .current_dir(&dir_git)
            .stdout(Stdio::null())
            .status()?;

        let rit_dir = dir_rit.join(".git");
        let file1_path = Path::new("objects/cc/628ccd10742baea8241c5924df992b5c019f71");
        let tree_path = Path::new("objects/b0/30fc230ce2ccfe74eeec1617105b26311f0e4a");
        assert!(rit_dir.join(file1_path).exists());

        let git_dir = dir_git.join(".git");
        let file1_blob_rit = std::fs::read(rit_dir.join(file1_path))?;
        let file1_blob_git = std::fs::read(git_dir.join(file1_path))?;
        assert_eq!(file1_blob_rit, file1_blob_git);

        assert!(rit_dir.join(tree_path).exists());
        let tree_rit = std::fs::read(rit_dir.join(tree_path))?;
        let tree_git = std::fs::read(git_dir.join(tree_path))?;
        assert_eq!(tree_rit, tree_git);

        let generated_commit = String::from_utf8(
            Command::new("git")
                .arg("cat-file")
                .arg("-p")
                .arg(&commit_id.to_hex())
                .current_dir(&dir_rit)
                .output()?
                .stdout,
        )
        .unwrap();

        if let [tree, _, _, _, msg] = generated_commit.lines().collect::<Vec<_>>()[..] {
            assert_eq!(
                tree, "tree b030fc230ce2ccfe74eeec1617105b26311f0e4a",
                "Tree OID in commit did not match"
            );
            assert_eq!(msg, "test", "Commit message did not match");
        } else {
            panic!("Invalid commit: {}", generated_commit);
        }
        Ok(())
    }
}
