use crate::*;

use camino::Utf8Path;
use tempdir::TempDir;

#[test]
/// Use rit to init a directory as a repo. Check that the correct directories have been created.
fn rit_init() -> Result<()> {
    let dir = TempDir::new("")?;
    let dir = Utf8Path::from_path(dir.as_ref()).unwrap();
    Repo::init_default(dir)?;

    let git_dir = dir.join(".git");
    let obj_dir = git_dir.join("objects");
    let refs_dir = git_dir.join("refs");

    assert!(git_dir.is_dir());
    assert!(obj_dir.is_dir());
    assert!(refs_dir.is_dir());

    Ok(())
}
