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
