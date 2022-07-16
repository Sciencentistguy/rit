use crate::test::{COMMIT_EMAIL, COMMIT_NAME};
use crate::*;
use std::path::Path;
use std::process::{Command, Stdio};
use tempdir::TempDir;

#[test]
/// Create two temporary directories. Create "a/b/c.txt" in both. In one, use rit to
/// init a repo, add the file, and commit it. In the other, use Git.
///
/// The generated Trees and Blob should be identical. The commit itself will not be identical due
/// to differing timestamps, but the *text* of the commit should be.
pub(super) fn rit_commit_hierarchy() -> Result<()> {
    let dir_rit = TempDir::new("")?;
    let dir_rit = dir_rit.path();
    let dir_git = TempDir::new("")?;
    let dir_git = dir_git.path();

    std::env::set_var("RIT_AUTHOR_NAME", COMMIT_NAME);
    std::env::set_var("RIT_AUTHOR_EMAIL", COMMIT_EMAIL);

    let git_command_args = [
        "-c",
        &format!("user.name={}", COMMIT_NAME),
        "-c",
        &format!("user.email={}", COMMIT_EMAIL),
        "-c",
        "commit.gpgsign=false",
    ];

    Repo::init(dir_rit)?;
    let mut rit_repo = Repo::open(dir_rit.to_owned())?;

    // Test files:
    // - a/b/c.txt: a file in a directory
    crate::create_test_files!(dir_rit, ["a/b/c.txt"]);
    rit_repo.add(&[".".into()])?;
    let commit_id = rit_repo.commit("test")?;

    Command::new("git")
        .args(&git_command_args)
        .arg("init")
        .current_dir(&dir_git)
        .stdout(Stdio::null())
        .status()
        .unwrap();

    crate::create_test_files!(dir_git, ["a/b/c.txt"]);

    Command::new("git")
        .args(&git_command_args)
        .arg("add")
        .arg("--all")
        .current_dir(&dir_git)
        .stdout(Stdio::null())
        .status()?;
    Command::new("git")
        .args(&git_command_args)
        .arg("commit")
        .arg("-m")
        .arg("test")
        .current_dir(&dir_git)
        .stdout(Stdio::null())
        .status()?;

    let rit_dir = dir_rit.join(".git");
    let root_tree_path = Path::new("objects/86/fd91b1c8d427d3577466833d9d686e85cd48df");
    let a_tree_path = Path::new("objects/4b/fb9c1f612da47abcfd8dfaff81dd7466b8f51e");
    let b_tree_path = Path::new("objects/c3/b2f03652d76b13a2ddb3a5da088ce7b203b3c8");
    let c_path = Path::new("objects/bf/c88425b0e2f167af3f1cfa9db193edf752b13b");
    assert!(rit_dir.join(root_tree_path).exists());
    assert!(rit_dir.join(a_tree_path).exists());
    assert!(rit_dir.join(b_tree_path).exists());
    assert!(rit_dir.join(c_path).exists());

    let git_dir = dir_git.join(".git");
    let c_blob_rit = std::fs::read(rit_dir.join(c_path))?;
    let c_blob_git = std::fs::read(git_dir.join(c_path))?;
    assert_eq!(c_blob_rit, c_blob_git);

    assert!(rit_dir.join(root_tree_path).exists());
    let tree_rit = std::fs::read(rit_dir.join(root_tree_path))?;
    let tree_git = std::fs::read(git_dir.join(root_tree_path))?;
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
            tree, "tree 86fd91b1c8d427d3577466833d9d686e85cd48df",
            "Tree OID in commit did not match"
        );
        assert_eq!(msg, "test", "Commit message did not match");
    } else {
        panic!("Invalid commit: {}", generated_commit);
    }
    Ok(())
}
