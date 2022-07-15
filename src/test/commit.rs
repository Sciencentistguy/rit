use crate::repo::*;
use crate::test::{COMMIT_EMAIL, COMMIT_NAME};
use crate::*;
use std::fs::Permissions;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use tempdir::TempDir;

#[test]
/// Create two temporary directories. Create the same set of files in both. In one, use rit to
/// init a repo, add the files, and commit them. In the other, use Git.
///
/// The generated Trees and Blobs should be identical. The commit itself will not be identical due
/// to differing timestamps, but the *text* of the commit should be.
fn commit() -> Result<()> {
    fn write_test_files(path: &Path) -> io::Result<()> {
        // Test files:
        // - file1: a normal file, chmod 644 (should be stored as REGULAR)
        // - file2: a normal file, chmod 755 (should be stored as EXECUTABLE)
        // - file3: a normal file, chmod 655 (should be stored as REGULAR (644))
        crate::create_test_files!(path, ["file1", "file2", "file3"]);
        std::fs::set_permissions(path.join("file2"), Permissions::from_mode(0o100755))?;
        std::fs::set_permissions(path.join("file3"), Permissions::from_mode(0o100655))?;
        Ok(())
    }
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

    let mut rit_repo = Repo::open(dir_rit.to_owned());

    rit_repo.init()?;
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
    let file1_path = Path::new("objects/86/f8ad067d20fa9b45f673d8e39f0bd9696664cb");
    let tree_path = Path::new("objects/b1/241e4ad46f3749d7c7962c122c5343dc2b90e4");
    assert!(rit_dir.join(file1_path).exists());

    let git_dir = dir_git.join(".git");
    let file1_blob_rit = std::fs::read(rit_dir.join(file1_path))?;
    let file1_blob_git = std::fs::read(git_dir.join(file1_path))?;
    assert_eq!(file1_blob_rit, file1_blob_git);

    assert!(rit_dir.join(tree_path).exists());
    let tree_rit = std::fs::read(rit_dir.join(tree_path))?;
    let tree_git = std::fs::read(git_dir.join(tree_path))?;
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
            tree, "tree b1241e4ad46f3749d7c7962c122c5343dc2b90e4",
            "Tree OID in commit did not match"
        );
        assert_eq!(msg, "test", "Commit message did not match");
        let generated_tree = String::from_utf8(
            Command::new("git")
                .arg("cat-file")
                .arg("-p")
                .arg(&tree[5..])
                .current_dir(&dir_rit)
                .output()?
                .stdout,
        )
        .unwrap();

        let file3_line = generated_tree.lines().nth(2).unwrap();
        let file3_line = file3_line.trim();

        assert!(file3_line.ends_with("file3"));
        assert_eq!(&file3_line[..6], "100644");
    } else {
        panic!("Invalid commit: {}", generated_commit);
    }

    Ok(())
}
