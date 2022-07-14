use std::io::Write;

use tempdir::TempDir;

use crate::repo::Repo;
use crate::storable::tree::TreeEntry;
use crate::Result;

#[test]
/// Create files "file1" and "file2". Add these to the index. Then, delete "file1", and create
/// the file "file1/file3" (a directory). Then, add this new file to the index.
///
/// The file "file1" should no longer be present in the index, as it cannot exist due to the
/// existance of "file1/file3"
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
    writeln!(std::fs::File::create(root.join("file1/file3"))?, "hello")?;

    repo.add(&["file1/file3".into()])?;

    let expected = ["file1/file3", "file2"];
    assert_eq!(repo.index.entries().len(), expected.len());

    for (actual, expected) in repo
        .index
        .entries()
        .iter()
        .map(|e| e.path().as_os_str().to_str().unwrap())
        .zip(expected)
    {
        assert_eq!(actual, expected);
    }

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
