use crate::*;
use tempdir::TempDir;

#[test]
/// Use rit to init a directory as a repo. Check that the correct directories have been created.
fn rit_init() -> Result<()> {
    let dir = TempDir::new("")?;
    let mut repo = Repo::open(dir.path().to_owned());
    repo.init()?;

    let git_dir = dir.as_ref().join(".git");
    let obj_dir = git_dir.join("objects");
    let refs_dir = git_dir.join("refs");

    assert!(git_dir.is_dir());
    assert!(obj_dir.is_dir());
    assert!(refs_dir.is_dir());

    Ok(())
}
