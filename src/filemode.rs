#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct FileMode(pub u32);

impl std::fmt::Octal for FileMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:o}", self.0)
    }
}

impl FileMode {
    pub const DIRECTORY: FileMode = FileMode(0o040000);
    pub const EXECUTABLE: FileMode = FileMode(0o100755);
    pub const REGULAR: FileMode = FileMode(0o100644);

    pub fn is_executable(self) -> bool {
        self.0 & libc::S_IXUSR != 0
    }
}
