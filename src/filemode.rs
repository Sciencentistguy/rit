use std::ops::{Deref, DerefMut};

use tracing::warn;

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

    #[cfg(target_os = "macos")]
    pub fn is_executable(self) -> bool {
        self.0 & libc::S_IXUSR as u32 != 0
    }

    #[cfg(target_os = "linux")]
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

impl From<&libc::stat> for FileMode {
    fn from(stat: &libc::stat) -> Self {
        #[cfg(target_os = "macos")]
        let actual_mode = FileMode(stat.st_mode as u32);

        #[cfg(target_os = "linux")]
        let actual_mode = FileMode(stat.st_mode);

        if actual_mode != FileMode::REGULAR && actual_mode != FileMode::EXECUTABLE {
            warn!(
                mode=?actual_mode,
                "Discarding information! Storing file with unsupported mode"
            );
        }

        if actual_mode.is_executable() {
            FileMode::EXECUTABLE
        } else {
            FileMode::REGULAR
        }
    }
}
