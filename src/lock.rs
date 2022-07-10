use crate::Result;

use std::{
    fs::File,
    io::{ErrorKind, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

pub struct LockedFile {
    guarded_path: PathBuf,
    lock_path: PathBuf,
    lockfile: File,
}

impl Deref for LockedFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.lockfile
    }
}
impl DerefMut for LockedFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lockfile
    }
}

impl LockedFile {
    pub fn try_aquire<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        let lock_path = path.as_ref().parent().expect("Cannot lock root").join(
            path.as_ref()
                .file_name()
                .expect("Cannot lock root")
                .to_string_lossy()
                .into_owned()
                + ".lock",
        );

        match File::options()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(None),
            Err(e) => Err(e.into()),
            Ok(lockfile) => Ok(Some(Self {
                guarded_path: path.as_ref().to_owned(),
                lock_path,
                lockfile,
            })),
        }
    }
}

impl Drop for LockedFile {
    fn drop(&mut self) {
        self.lockfile.flush().unwrap();
        std::fs::rename(&self.lock_path, &self.guarded_path).unwrap();
    }
}
