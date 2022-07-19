use std::ffi::CString;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::{io::Write, os::unix::prelude::OsStrExt};

use rand::prelude::*;
use rayon::prelude::*;
use tempdir::TempDir;

use crate::{
    repo::Repo,
    test::{COMMIT_EMAIL, COMMIT_NAME},
    Result,
};

fn init_repo(dir: &Path) -> Result<Repo> {
    std::env::set_var("RIT_AUTHOR_NAME", COMMIT_NAME);
    std::env::set_var("RIT_AUTHOR_EMAIL", COMMIT_EMAIL);

    crate::create_test_files!(dir, ["file1", "file2", "file3", "file4"]);

    Repo::init(dir)?;

    let mut repo = Repo::open(dir.to_owned())?;
    repo.add(&[".".into()])?;
    repo.commit("test")?;
    Ok(repo)
}

#[test]
fn test_untracked() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();

    let repo = init_repo(dir)?;

    {
        let (files, index) = repo.read_status()?;
        assert_eq!(repo.untracked_files(&files, &index).count(), 0);
    }

    crate::create_test_files!(dir, ["file5", "file6", "file7", "file8"]);

    {
        let (files, index) = repo.read_status()?;
        assert_eq!(repo.untracked_files(&files, &index).count(), 4);
    }

    Ok(())
}

#[test]
fn test_change_file_contents() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();

    let repo = init_repo(dir)?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 0);
    }

    write!(
        std::fs::File::options()
            .append(true)
            .open(dir.join("file1"))?,
        "-changed"
    )?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 1);
    }

    Ok(())
}

#[test]
fn test_change_file_mode() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();

    let repo = init_repo(dir)?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 0);
    }

    {
        let permissions = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(dir.join("file1"), permissions)?;
    }

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 1);
    }

    Ok(())
}

#[test]
fn test_change_file_preserve_size() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();

    let repo = init_repo(dir)?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 0);
    }

    {
        let len = std::fs::File::open(dir.join("file1"))?.metadata()?.len();
        let mut new_contents: Vec<u8> = Vec::new();
        for _ in 0..len {
            new_contents.push(thread_rng().gen());
        }
        let mut file = std::fs::File::create(dir.join("file1"))?;
        file.write_all(&new_contents)?;
        drop(file);
        let new_len = std::fs::File::open(dir.join("file1"))?.metadata()?.len();
        assert_eq!(len, new_len);
    }

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 1);
    }

    Ok(())
}

#[test]
fn test_no_change_touched() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();

    let repo = init_repo(dir)?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 0);
    }

    filetime::set_file_mtime(dir.join("file1"), filetime::FileTime::now())?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.changed_files(&index).count(), 0);
    }

    Ok(())
}

#[test]
fn test_delete_file() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();

    let repo = init_repo(dir)?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.deleted_files(&index).count(), 0);
    }

    std::fs::remove_file(dir.join("file1"))?;

    {
        let (_, index) = repo.read_status()?;
        assert_eq!(repo.deleted_files(&index).count(), 1);
    }

    Ok(())
}
