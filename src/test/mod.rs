mod add;
mod commit;
mod init;

pub const COMMIT_NAME: &str = "Jamie Quigley";
pub const COMMIT_EMAIL: &str = "jamie@quigley.xyz";

#[macro_export]
macro_rules! create_test_files {
    ($root:ident, [$($path:expr),*]) => {{
        use std::io::Write;
        $({
            let path = $root.join($path);
            std::fs::create_dir_all(path.parent().unwrap())?;
            writeln!(
                std::fs::File::create($root.join($path))?,
                concat!(stringify!($path), "-contents")
                )?;
        })*
    }};
}
