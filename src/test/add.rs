use std::io::Write;

use tempdir::TempDir;

use crate::repo::Repo;
use crate::storable::tree::TreeEntry;
use crate::Result;

#[test]
fn test_dir_replaces_file() -> Result<()> {
    let root = TempDir::new("").unwrap();
    let root = root.path();
    let mut repo = Repo::open(root.to_owned());

    repo.init()?;
    writeln!(std::fs::File::create(root.join("file1"))?, "hello")?;
    writeln!(std::fs::File::create(root.join("file2"))?, "world")?;
    repo.add(&["file1".into(), "file2".into()])?;

    std::fs::remove_file(root.join("file1"))?;
    std::fs::create_dir(root.join("file1"))?;
    writeln!(std::fs::File::create(root.join("file1/file1"))?, "hello")?;

    repo.add(&["file1/file1".into()])?;

    let actual = repo.index.entries()[0].path().strip_prefix(root)?;
    let actual = actual.as_os_str().to_str().unwrap();

    assert_eq!(actual, "file1/file1");

    Ok(())
}

#[test]
fn test_file_replaces_dir() -> Result<()> {
    let root = TempDir::new("").unwrap();
    let root = root.path();
    let mut repo = Repo::open(root.to_owned());

    repo.init()?;
    std::fs::create_dir(root.join("file1"))?;
    writeln!(std::fs::File::create(root.join("file1/file1"))?, "hello")?;
    writeln!(std::fs::File::create(root.join("file2"))?, "world")?;
    repo.add(&["file1/file1".into(), "file2".into()])?;
    // dbg!(&repo.index);
    for entry in repo.index.entries() {
        let name = std::str::from_utf8(entry.name())?;
        println!("{:?}", name);
    }
    std::fs::remove_dir_all(root.join("file1"))?;
    writeln!(std::fs::File::create(root.join("file1"))?, "hello")?;

    repo.add(&["file1".into()])?;
    println!();

    for entry in repo.index.entries() {
        let name = std::str::from_utf8(entry.name())?;
        println!("{:?}", name);
    }

    assert!(!repo
        .index
        .entries()
        .iter()
        .any(|entry| entry.name().ends_with(b"file1/file1")));

    let actual = repo.index.entries()[0].path().strip_prefix(root)?;
    let actual = actual.as_os_str().to_str().unwrap();

    assert_eq!(actual, "file1");

    Ok(())
}

// #[test]
// fn test_dir_replaces_file() -> Result<()> {
// let root = TempDir::new("").unwrap();
// let root = root.path();
// let mut repo = Repo::open(root.to_owned());

// repo.init()?;
// writeln!(std::fs::File::create(root.join("file1"))?, "hello")?;
// writeln!(std::fs::File::create(root.join("file2"))?, "world")?;
// repo.add(&["file1".into(), "file2".into()])?;

// std::fs::remove_file(root.join("file1"))?;
// std::fs::create_dir(root.join("file1"))?;
// writeln!(std::fs::File::create(root.join("file1/file1"))?, "hello")?;

// repo.add(&["file1/file1".into()])?;

// let actual = repo.index.entries()[0].path().strip_prefix(root)?;
// let actual = actual.as_os_str().to_str().unwrap();

// assert_eq!(actual, "file1/file1");

// Ok(())
// }
