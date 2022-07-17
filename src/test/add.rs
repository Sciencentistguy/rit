use tempdir::TempDir;

use crate::repo::Repo;
use crate::storable::tree::TreeEntry;
use crate::Result;
use pretty_assertions::assert_eq;

#[test]
/// Create files "file1" and "file2". Add these to the index. Then, delete "file1", and create
/// the file "file1/file3" (a directory). Then, add this new file to the index.
///
/// The file "file1" should no longer be present in the index, as it cannot exist due to the
/// existance of "file1/file3"
fn test_dir_replaces_file() -> Result<()> {
    let root = TempDir::new("").unwrap();
    let root = root.path();

    Repo::init(root)?;
    let mut repo = Repo::open(root.to_owned())?;

    crate::create_test_files!(root, ["file1", "file2"]);
    repo.add(&["file1".into(), "file2".into()])?;

    std::fs::remove_file(root.join("file1"))?;
    crate::create_test_files!(root, ["file1/file3"]);
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
/// Add a file to the index with the same name as a previously existing directory.
///
/// The directory, and all members of that directory, should be removed from the index.
fn test_file_replaces_dir() -> Result<()> {
    let root = TempDir::new("").unwrap();
    let root = root.path();

    Repo::init(root)?;
    let mut repo = Repo::open(root.to_owned())?;

    crate::create_test_files!(root, ["file1/file2/file3", "file1/file2/file4", "file5"]);
    repo.add(&[
        "file1/file2/file3".into(),
        "file1/file2/file4".into(),
        "file5".into(),
    ])?;

    println!("after first add:");
    for entry in repo.index.entries() {
        let name = std::str::from_utf8(entry.name())?;
        println!("{:?}", name);
    }
    std::fs::remove_dir_all(root.join("file1"))?;
    crate::create_test_files!(root, ["file1"]);
    repo.add(&["file1".into()])?;

    println!("after second add:");
    for entry in repo.index.entries() {
        let name = std::str::from_utf8(entry.name())?;
        println!("{:?}", name);
    }

    let expected = ["file1", "file5"];
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
