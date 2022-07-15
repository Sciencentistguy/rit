use std::ops::{Deref, DerefMut};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FileMode(pub u32);

impl std::fmt::Octal for FileMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:o}", self.0)
    }
}

impl std::fmt::Debug for FileMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileMode(0o{:o})", self.0)
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

impl From<u32> for FileMode {
    fn from(mode: u32) -> Self {
        Self(mode)
    }
}

impl Deref for FileMode {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FileMode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
