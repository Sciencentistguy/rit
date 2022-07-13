use crate::*;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use tempdir::TempDir;

#[test]
pub(super) fn rit_commit_hierarchy() -> Result<()> {
    fn write_test_files(path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path.join("a/b"))?;
        writeln!(std::fs::File::create(path.join("a/b/c.txt"))?, "file")?;
        Ok(())
    }
    let dir_rit = TempDir::new("")?;
    let dir_rit = dir_rit.path();
    let dir_git = TempDir::new("")?;
    let dir_git = dir_git.path();

    const COMMIT_NAME: &str = "Jamie Quigley";
    const COMMIT_EMAIL: &str = "jamie@quigley.xyz";

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

    let mut rit_repo = Repo::open(dir_rit.to_owned());

    // Rit create files
    rit_repo.init()?;
    // create test files
    write_test_files(dir_rit)?;
    rit_repo.add(&[".".into()])?;
    let commit_id = rit_repo.commit("test")?;

    Command::new("git")
        .args(&git_command_args)
        .arg("init")
        .current_dir(&dir_git)
        .stdout(Stdio::null())
        .status()
        .unwrap();

    write_test_files(dir_git)?;

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
    let root_tree_path = Path::new("objects/43/f40e5accf591f2187d45ed0b2458d687a13554");
    let a_tree_path = Path::new("objects/d8/ee74535df69d1cb6f4fc16a5a36d0f71ea948f");
    let b_tree_path = Path::new("objects/ba/1c8f19d3c39b63be0a4af9e3c903c5417573a5");
    let c_path = Path::new("objects/f7/3f3093ff865c514c6c51f867e35f693487d0d3");
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
            tree, "tree 43f40e5accf591f2187d45ed0b2458d687a13554",
            "Tree OID in commit did not match"
        );
        assert_eq!(msg, "test", "Commit message did not match");
    } else {
        panic!("Invalid commit: {}", generated_commit);
    }
    Ok(())
}
