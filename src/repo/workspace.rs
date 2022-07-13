use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;

use tracing::*;
use walkdir::WalkDir;

use crate::storable::{blob::Blob, tree::Entry, Storable as _};
use crate::*;

impl super::Repo {
    pub fn list_files(&self) -> Result<Vec<Entry>> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(&self.dir) {
            let entry = entry?;
            let path = entry.path();
            if path
                .components()
                .any(|c| AsRef::<Path>::as_ref(&c) == Path::new(".git"))
            {
                continue;
            }
            trace!(?path, "Found entry");
            if !path.is_dir() {
                let data = std::fs::read(&path)?;
                let blob = Blob::new(&data);
                self.database.store(&blob)?;
                let metadata = std::fs::metadata(&path)?;
                entries.push(Entry::new(
                    path.strip_prefix(&self.dir)?.to_owned(),
                    blob.into_oid(),
                    metadata,
                ));
            }
        }
        Ok(entries)
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
