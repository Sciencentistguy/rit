#![allow(dead_code)]

mod database;
mod interface;
mod storable;
mod util;
mod workspace;

pub use color_eyre::Result;
use hex::ToHex;
use storable::commit::Author;
use storable::commit::Commit;
use storable::tree::Entry;
use storable::tree::Tree;

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::interface::*;
use crate::storable::blob::Blob;
use crate::storable::Storable;
use crate::workspace::Workspace;

use clap::Parser;
use once_cell::sync::Lazy;

static ARGS: Lazy<Opt> = Lazy::new(Opt::parse);
static ROOT: Lazy<PathBuf> = Lazy::new(|| match &ARGS.path {
    Some(x) => x.clone(),
    None => std::env::current_dir().expect("Process has no directory :thonk:"),
});

fn main() -> Result<()> {
    color_eyre::install().unwrap();
    Lazy::force(&ARGS);

    match &ARGS.command {
        Command::Init => init(&*ROOT)?,
        Command::Commit { message } => commit(
            &*ROOT,
            message
                .as_deref()
                .expect("Using an editor for commit message is currently unimplemented"),
        )?,
    }
    Ok(())
}

fn init<P: AsRef<Path>>(path: P) -> Result<()> {
    let dir = path.as_ref().join(".git");
    for d in ["objects", "refs"] {
        std::fs::create_dir_all(dir.join(d))?;
    }
    Ok(())
}

fn commit<P: AsRef<Path>>(root: P, message: &str) -> Result<()> {
    let root = root.as_ref();
    let wsp = Workspace::new(root);
    let database = database::Database::new(root.join(".git/objects"));
    let mut entries = Vec::new();
    for file in wsp.list_files()? {
        let filepath = root.join(file);
        let data = std::fs::read(&filepath)?;
        let blob = Blob::new(&data);
        database.store(&blob)?;
        entries.push(Entry::new(filepath.file_name().unwrap(), blob.into_oid()));
    }
    let tree = Tree::new(entries);
    database.store(&tree)?;

    let author = Author {
        name: std::env::var("RIT_AUTHOR_NAME")?,
        email: std::env::var("RIT_AUTHOR_EMAIL")?,
    };

    let commit = Commit::new(tree.into_oid(), author, message);
    database.store(&commit)?;
    File::create(root.join(".git/HEAD"))?
        .write_all(commit.get_oid().encode_hex::<String>().as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tempdir::TempDir;

    #[test]
    fn rit_init() -> Result<()> {
        let dir = TempDir::new("")?;
        super::init(&dir)?;

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
        let dir_rit = TempDir::new("")?;
        let dir_git = TempDir::new("")?;

        // Rit create files
        crate::init(&dir_rit)?;
        writeln!(
            std::fs::File::create(dir_rit.as_ref().join("file1"))?,
            "hello"
        )?;
        writeln!(
            std::fs::File::create(dir_rit.as_ref().join("file2"))?,
            "world"
        )?;
        crate::commit(&dir_rit, "test")?;

        let _ = Command::new("git")
            .arg("init")
            .current_dir(&dir_git)
            .status()?;
        let _ = Command::new("git")
            .arg("config")
            .arg("--local")
            .arg("commit.gpgsign")
            .arg("false")
            .current_dir(&dir_git)
            .status()?;
        writeln!(
            std::fs::File::create(dir_git.as_ref().join("file1"))?,
            "hello"
        )?;
        writeln!(
            std::fs::File::create(dir_git.as_ref().join("file2"))?,
            "world"
        )?;
        Command::new("git")
            .arg("add")
            .arg("--all")
            .current_dir(&dir_git)
            .stdout(Stdio::null())
            .status()?;
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("test")
            .current_dir(&dir_git)
            .stdout(Stdio::null())
            .status()?;

        let rit_dir = dir_rit.as_ref().join(".git");
        let file1_path = Path::new("objects/cc/628ccd10742baea8241c5924df992b5c019f71");
        let tree_path = Path::new("objects/81/2bcf7a7db574cf24a2d6b8ed92cfd096c219e5");
        assert!(rit_dir.join(file1_path).exists());
        let git_dir = dir_git.as_ref().join(".git");
        let file1_blob_rit = std::fs::read(rit_dir.join(file1_path))?;
        let file1_blob_git = std::fs::read(git_dir.join(file1_path))?;
        assert_eq!(file1_blob_rit, file1_blob_git);

        assert!(rit_dir.join(tree_path).exists());
        let tree_rit = std::fs::read(rit_dir.join(tree_path))?;
        let tree_git = std::fs::read(git_dir.join(tree_path))?;
        assert_eq!(tree_rit, tree_git);

        // TODO check that there's a commit
        
        Ok(())
    }
}
