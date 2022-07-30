use crate::digest::Digest;

use std::{io, path::Path, process::Command};

mod add;
mod commit;
mod init;
mod status;

pub const COMMIT_NAME: &str = "Jamie Quigley";
pub const COMMIT_EMAIL: &str = "jamie@quigley.xyz";

#[macro_export]
macro_rules! test_file_contents {
    ($name:literal) => {
        concat!(stringify!($name), "-contents\n")
    };
}

#[macro_export]
macro_rules! create_test_files {
    ($root:ident, [$($path:expr),*]) => {{
        use std::io::Write;
        $({
            let path = $root.join($path);
            std::fs::create_dir_all(path.parent().unwrap())?;
            write!(std::fs::File::create($root.join($path))?, crate::test_file_contents!($path))?;
        })*
    }};
}

pub fn git_cat_file(dir: &Path, oid: &Digest) -> io::Result<String> {
    Ok(String::from_utf8(
        Command::new("git")
            .arg("cat-file")
            .arg("-p")
            .arg(&oid.to_hex())
            .current_dir(&dir)
            .output()?
            .stdout,
    )
    .unwrap())
}
