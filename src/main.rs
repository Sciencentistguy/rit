#![allow(dead_code)]

mod database;
mod interface;
mod storable;
mod util;
mod workspace;

pub use color_eyre::Result;

use std::path::Path;
use std::path::PathBuf;

use crate::interface::*;
use crate::storable::Blob;
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

    match ARGS.command {
        Command::Init => init(&*ROOT)?,
        Command::Commit => commit(&*ROOT)?,
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

fn commit<P: AsRef<Path>>(root: P) -> Result<()> {
    let root = root.as_ref();
    let git_path = root.join(".git");
    let db_path = git_path.join("objects");
    let wsp = Workspace::new(root);
    let database = database::Database::new(db_path);
    for file in wsp.list_files()? {
        let filepath = root.join(file);
        let data = std::fs::read(filepath)?;
        let blob = Blob::new(&data);
        database.store(&blob)?;
    }
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
        crate::commit(&dir_rit)?;

        let _ = Command::new("git")
            .arg("init")
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
        assert!(rit_dir.join(file1_path).exists());
        let git_dir = dir_git.as_ref().join(".git");
        let file1_blob_rit = std::fs::read(rit_dir.join(file1_path))?;
        let file1_blob_git = std::fs::read(git_dir.join(file1_path))?;

        assert_eq!(file1_blob_rit, file1_blob_git);

        Ok(())
    }
}
