use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::unix::prelude::OsStrExt;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{eyre, Context};
use walkdir::WalkDir;

use crate::*;

impl super::Repo {
    pub fn list_files(&self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        let path = self.dir.join(path);
        if path.is_file() {
            Ok(vec![path])
        } else {
            let ignores = self.ignores();

            let mut entries = Vec::new();

            'outer: for entry in WalkDir::new(path) {
                let entry = entry?;
                let path = entry.path();
                let path = Utf8Path::from_path(path).ok_or_else(|| {
                    eyre!(
                        "All paths must be valid unicode: found '{:?}'",
                        path.display()
                    )
                })?;

                for ignore in ignores {
                    if path.as_str().contains(ignore) {
                        continue 'outer;
                    }
                }

                if path.is_dir() && !path.is_symlink() {
                    continue 'outer;
                }
                entries.push(path.strip_prefix(&self.dir)?.to_owned());
            }
            Ok(entries)
        }
    }

    /// Get the libc::stat information for a file. Returns None if the file does not exist
    pub fn stat_file(path: &Utf8Path) -> Result<Option<libc::stat>> {
        if path.exists() {
            // Safety: Calls libc::stat. Stat doesn't read from its second argument, so this is
            // sound
            unsafe {
                let mut dest: MaybeUninit<libc::stat> = MaybeUninit::uninit();
                let cpath = CString::new(path.as_os_str().as_bytes()).unwrap();
                let err = libc::stat(cpath.as_ptr(), dest.as_mut_ptr());
                match err {
                    0 => Ok(Some(dest.assume_init())),
                    -1 => {
                        let error = std::io::Error::last_os_error();
                        Err(error).wrap_err_with(|| format!("libc::stat({path}) failed"))
                    }
                    _ => unreachable!("libc::stat cannot return other values"),
                }
            }
        } else {
            Ok(None)
        }
    }
}
