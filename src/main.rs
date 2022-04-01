#![allow(dead_code)]

mod database;
mod digest;
mod interface;
mod refs;
mod storable;
mod util;
mod workspace;
mod lock;

pub use color_eyre::Result;

use database::Database;
use digest::Digest;
use interface::*;
use refs::Refs;
use storable::blob::Blob;
use storable::commit::Author;
use storable::commit::Commit;
use storable::tree::Entry;
use storable::tree::Tree;
use storable::Storable;
use workspace::Workspace;

use std::path::Path;
use std::path::PathBuf;

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
        Command::Commit { message } => {
            let commit_id = commit(
                &*ROOT,
                message
                    .as_deref()
                    .expect("Using an editor for commit message is currently unimplemented"),
            )?;
            println!("Created commit {}", commit_id.to_hex())
        }
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

fn commit<P: AsRef<Path>>(root: P, message: &str) -> Result<Digest> {
    let root = root.as_ref();
    let wsp = Workspace::new(root);
    let refs = Refs::new(root);
    let database = Database::new(root);
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

    let parent_commit = refs.read_head()?;

    let author = Author {
        name: std::env::var("RIT_AUTHOR_NAME")?,
        email: std::env::var("RIT_AUTHOR_EMAIL")?,
    };

    let commit = Commit::new(parent_commit, tree.into_oid(), author, message);
    database.store(&commit)?;
    refs.set_head(commit.get_oid())?;

    Ok(commit.into_oid())
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
        let commit_id = crate::commit(&dir_rit, "test")?;

        let _ = Command::new("git")
            .arg("init")
            .current_dir(&dir_git)
            .stdout(Stdio::null())
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

        if let [tree, _author, _committer, _, msg] =
            generated_commit.lines().collect::<Vec<_>>()[..]
        {
            assert_eq!(tree, "tree 812bcf7a7db574cf24a2d6b8ed92cfd096c219e5");
            assert_eq!(msg, "test");
        } else {
            panic!("Invalid commit: {}", generated_commit);
        }
        Ok(())
    }
}
