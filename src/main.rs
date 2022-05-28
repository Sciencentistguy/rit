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

fn open_repo<P: AsRef<Path>>(root_path: P) -> (Workspace, Refs, Database) {
    let root_path = root_path.as_ref();
    let workspace = Workspace::new(root_path);
    let refs = Refs::new(root_path);
    let database = Database::new(root_path);
    (workspace, refs, database)
}

fn init<P: AsRef<Path>>(path: P) -> Result<()> {
    let dir = path.as_ref().join(".git");
    for d in ["objects", "refs"] {
        std::fs::create_dir_all(dir.join(d))?;
    }
    Ok(())
}

fn commit<P: AsRef<Path>>(root: P, message: &str) -> Result<Digest> {
    let (workspace, refs, database) = open_repo(root);
    let mut entries = Vec::new();
    for filepath in workspace.list_files()? {
        let data = std::fs::read(&filepath)?;
        let blob = Blob::new(&data);
        database.store(&blob)?;
        let metadata = std::fs::metadata(&filepath)?;
        entries.push(Entry::new(
            filepath.file_name().unwrap(),
            blob.into_oid(),
            metadata,
        ));
    }
    let tree = Tree::new(entries);
    database.store(&tree)?;
    dbg!(tree.get_oid());

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
    use std::fs::Permissions;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
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
        let dir_rit = dir_rit.as_ref();
        let dir_git = TempDir::new("")?;
        let dir_git = dir_git.as_ref();

        std::env::set_var("RIT_AUTHOR_NAME", "Jamie Quigley");
        std::env::set_var("RIT_AUTHOR_EMAIL", "jamie@quigley.xyz");

        // Rit create files
        crate::init(&dir_rit)?;
        writeln!(std::fs::File::create(dir_rit.join("file1"))?, "hello")?;
        writeln!(std::fs::File::create(dir_rit.join("file2"))?, "world")?;
        std::fs::set_permissions(dir_rit.join("file2"), Permissions::from_mode(0o100755))?;
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
        writeln!(std::fs::File::create(dir_git.join("file1"))?, "hello")?;
        writeln!(std::fs::File::create(dir_git.join("file2"))?, "world")?;
        std::fs::set_permissions(dir_git.join("file2"), Permissions::from_mode(0o100755))?;

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
