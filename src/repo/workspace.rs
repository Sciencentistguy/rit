use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::*;

impl super::Repo {
    pub fn list_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let path = self.dir.join(path);
        if path.is_file() {
            Ok(vec![path])
        } else {
            let mut entries = Vec::new();

            for entry in WalkDir::new(path) {
                let entry = entry?;
                let path = entry.path();
                if path
                    .components()
                    .any(|c| AsRef::<Path>::as_ref(&c) == Path::new(".git"))
                {
                    continue;
                }
                if path.is_dir() && !path.is_symlink() {
                    continue;
                }
                entries.push(path.strip_prefix(&self.dir)?.to_owned());
            }
            Ok(entries)
        }
    }

    pub fn stat_file(path: &Path) -> libc::stat {
        // Safety: Calls libc::stat. Stat doesn't read from its second argument, so this is sound
        unsafe {
            let mut dest: MaybeUninit<libc::stat> = MaybeUninit::uninit();
            let path = CString::new(path.as_os_str().as_bytes()).unwrap();
            let err = libc::stat(path.as_ptr(), dest.as_mut_ptr());
            if err != 0 {
                panic!("stat failed with code: {}", err);
            }
            dest.assume_init()
        }
    }
}
