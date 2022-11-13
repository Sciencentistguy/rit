use libc::mode_t;
use tracing::warn;

/// A file mode.
///
/// Rit only acknowledges the existence of 3 types of file mode (for now...):
///  - regular files (0o100644)
///  - executable files (0o100755)
///  - directories (0o040000)
///
/// As fewer than 256 states are actually represented, we can save 3 bytes by not storing the whole
/// mode as a `mode_t`. Ensure that
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileMode {
    Directory,
    Executable,
    Regular,
}

impl std::fmt::Octal for FileMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:o}", self.inner())
    }
}

impl FileMode {
    const DIRECTORY: mode_t = 0o040000;
    const EXECUTABLE: mode_t = 0o100755;
    const REGULAR: mode_t = 0o100644;

    pub fn inner(&self) -> u32 {
        (match self {
            FileMode::Directory => FileMode::DIRECTORY,
            FileMode::Executable => FileMode::EXECUTABLE,
            FileMode::Regular => FileMode::REGULAR,
        }) as _
    }

    /// Returns `true` if the file mode is [`Executable`].
    ///
    /// [`Executable`]: FileMode::Executable
    #[must_use]
    pub fn is_executable(&self) -> bool {
        matches!(self, Self::Executable)
    }
}

impl From<mode_t> for FileMode {
    fn from(native_mode: mode_t) -> Self {
        match native_mode {
            FileMode::DIRECTORY => FileMode::Directory,
            FileMode::REGULAR => FileMode::Regular,
            FileMode::EXECUTABLE => FileMode::Executable,
            actual_mode => {
                warn!(
                    mode=?actual_mode,
                    "Discarding information! Storing file with unsupported mode"
                );
                if actual_mode & libc::S_IXUSR != 0 {
                    FileMode::Executable
                } else {
                    FileMode::Regular
                }
            }
        }
    }
}

impl From<&libc::stat> for FileMode {
    fn from(stat: &libc::stat) -> Self {
        stat.st_mode.into()
    }
}

#[test]
fn size() {
    assert_eq!(std::mem::size_of::<FileMode>(), 1);
}
