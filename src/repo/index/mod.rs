mod parse;
mod write;

use std::ffi::CStr;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};

use crate::digest::Digest;
use crate::Result;

struct IndexHeader {
    magic: [u8; 4],
    version: u32,
    num_entries: u32,
}

impl IndexHeader {
    fn has_valid_magic(&self) -> bool {
        &self.magic == b"DIRC"
    }
}

struct IndexEntry<'a> {
    ctime_s: u32,
    ctime_n: u32,

    mtime_s: u32,
    mtime_n: u32,

    dev: u32,
    ino: u32,

    mode: u32,

    uid: u32,
    gid: u32,
    siz: u32,
    oid: Digest,
    flags: u16,
    name: &'a CStr,
}

impl IndexEntry<'_> {
    const MAX_PATH_SIZE: u16 = 0xfff;

    fn create(path: &Path, oid: Digest, stat: libc::stat) -> Result<Self> {
        let name = path
            .file_name()
            .expect("File should have name")
            .as_bytes()
            .to_owned();

        let flags = path
            .as_os_str()
            .len()
            .try_into()
            .unwrap_or(Self::MAX_PATH_SIZE);

        Ok(Self {
            ctime_s: stat.st_ctime.try_into()?,
            ctime_n: stat.st_ctime_nsec.try_into()?,
            mtime_s: stat.st_mtime.try_into()?,
            mtime_n: stat.st_mtime_nsec.try_into()?,
            dev: stat.st_dev.try_into()?,
            ino: stat.st_ino.try_into()?,
            mode: stat.st_mode,
            uid: stat.st_uid,
            gid: stat.st_gid,
            siz: stat.st_size.try_into()?,
            oid,
            flags,
            name: todo!(),
        })
    }
}

struct Index<'a> {
    header: IndexHeader,
    entries: Vec<IndexEntry<'a>>,
    oid: Digest,
}

pub struct IndexWrapper {
    path: PathBuf,
    // entries: Vec<IndexEntry>,
}

impl IndexWrapper {
    pub fn open(path: &Path) -> Self {
        Self {
            path: path.join(".git/index"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse::*, write::*};

    #[test]
    #[ignore = "Doesn't work in CI"]
    fn read_write_index() {
        // let bytes = std::fs::read("/dev/shm/rit/.git/index").unwrap();
        let bytes = std::fs::read("/home/jamie/Git/nixpkgs-official/.git/index").unwrap();
        let idx = parse_index(&bytes);

        for e in &idx.entries {
            let name = e.name.to_str();
            println!("{:?}", name);
        }

        let new_bytes = write_index(&idx);
        if new_bytes.len() < bytes.len() {
            println!("Dropped extensions, oid will not match...");
            let len = new_bytes.len() - 20;
            assert_eq!(bytes[..len], new_bytes[..len]);
        } else {
            assert_eq!(bytes, new_bytes);
        }
    }
}
