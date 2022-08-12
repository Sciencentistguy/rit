use std::io::Write;
use std::os::unix::prelude::PermissionsExt;

use camino::Utf8Path;
use rand::prelude::*;
use tempdir::TempDir;

use crate::{
    repo::{
        status::{Change, Status},
        Repo,
    },
    test::{COMMIT_EMAIL, COMMIT_NAME},
    Result,
};

fn init_repo(dir: &Utf8Path) -> Result<Repo> {
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
    let dir = Utf8Path::from_path(dir).unwrap();

    let repo = init_repo(dir)?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 0);
    }

    crate::create_test_files!(dir, ["file5", "file6", "file7", "file8"]);

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 4);

        for (_, change) in files {
            assert_eq!(change, Change::Untracked);
        }
    }

    Ok(())
}

#[test]
fn test_change_file_contents() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();
    let dir = Utf8Path::from_path(dir).unwrap();

    let repo = init_repo(dir)?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 0);
    }

    write!(
        std::fs::File::options()
            .append(true)
            .open(dir.join("file1"))?,
        "-changed"
    )?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 1);

        for (_, change) in files {
            assert_eq!(change, Change::Modified);
        }
    }

    Ok(())
}

#[test]
fn test_change_file_mode() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();
    let dir = Utf8Path::from_path(dir).unwrap();

    let repo = init_repo(dir)?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 0);
    }

    {
        let permissions = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(dir.join("file1"), permissions)?;
    }

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 1);

        for (_, change) in files {
            assert_eq!(change, Change::Modified);
        }
    }

    Ok(())
}

#[test]
fn test_change_file_preserve_size() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();
    let dir = Utf8Path::from_path(dir).unwrap();

    let repo = init_repo(dir)?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 0);
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
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 1);

        for (_, change) in files {
            assert_eq!(change, Change::Modified);
        }
    }

    Ok(())
}

#[test]
fn test_no_change_touched() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();
    let dir = Utf8Path::from_path(dir).unwrap();

    let repo = init_repo(dir)?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 0);
    }

    filetime::set_file_mtime(dir.join("file1"), filetime::FileTime::now())?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 0);
    }

    Ok(())
}

#[test]
fn test_delete_file() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = dir.path();
    let dir = Utf8Path::from_path(dir).unwrap();

    let repo = init_repo(dir)?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 0);
    }

    std::fs::remove_file(dir.join("file1"))?;

    {
        let status = Status::new(&repo)?.unwrap();
        let files = status.get_statuses()?;
        assert_eq!(files.len(), 1);

        for (_, change) in files {
            assert_eq!(change, Change::Removed);
        }
    }

    Ok(())
}
