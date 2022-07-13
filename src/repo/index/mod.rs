mod parse;
mod write;

use std::ffi::OsStr;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};

use tracing::trace;

use crate::digest::Digest;
use crate::filemode::FileMode;
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IndexEntry {
    ctime_s: u32,
    ctime_n: u32,

    mtime_s: u32,
    mtime_n: u32,

    dev: u32,
    ino: u32,

    mode: FileMode,

    uid: u32,
    gid: u32,
    siz: u32,
    oid: Digest,
    flags: u16,
    name: Vec<u8>,
}

impl crate::storable::tree::TreeEntry for IndexEntry {
    fn digest(&self) -> &Digest {
        &self.oid
    }

    fn mode(&self) -> FileMode {
        self.mode
    }

    fn name(&self) -> &[u8] {
        self.name.as_ref()
    }

    fn path(&self) -> &Path {
        Path::new(OsStr::from_bytes(self.name.as_ref()))
    }
}

impl Ord for IndexEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}
impl PartialOrd for IndexEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl IndexEntry {
    const MAX_PATH_SIZE: u16 = 0xfff;

    fn create(path: &Path, oid: &Digest, stat: libc::stat) -> Result<Self> {
        let name = path
            // .file_name()
            // .expect("File should have name")
            .as_os_str()
            .as_bytes()
            .to_owned();

        let flags = path
            .as_os_str()
            .len()
            .try_into()
            .unwrap_or(Self::MAX_PATH_SIZE);

        let mode = FileMode(stat.st_mode);
        let mode = if mode.is_executable() {
            FileMode::EXECUTABLE
        } else {
            FileMode::REGULAR
        };

        Ok(Self {
            ctime_s: stat.st_ctime.try_into()?,
            ctime_n: stat.st_ctime_nsec.try_into()?,
            mtime_s: stat.st_mtime.try_into()?,
            mtime_n: stat.st_mtime_nsec.try_into()?,
            dev: stat.st_dev.try_into()?,
            ino: stat.st_ino.try_into()?,
            mode,
            uid: stat.st_uid,
            gid: stat.st_gid,
            siz: stat.st_size.try_into()?,
            oid: oid.clone(),
            flags,
            name,
        })
    }
}

struct Index {
    header: IndexHeader,
    entries: Vec<IndexEntry>,
    // oid: Digest,
}

impl Index {
    fn from_entries(entries: &[IndexEntry]) -> Self {
        let header = IndexHeader {
            magic: *b"DIRC",
            version: 2,
            num_entries: entries
                .len()
                .try_into()
                .expect("The number of entries should fit in a u32"),
        };

        Self {
            header,
            entries: entries.to_owned(),
        }
    }
}

pub struct IndexWrapper {
    path: PathBuf,
    //FIXME: this could be a Cow
    entries: Vec<IndexEntry>,
}

impl IndexWrapper {
    pub fn open(path: &Path) -> Self {
        let path = path.join(".git/index");
        let entries = (|| -> Result<Vec<IndexEntry>> {
            let current_index = std::fs::read(&path)?;
            let current_index = parse::parse_index(&current_index);
            Ok(current_index.entries)
        })()
        .unwrap_or_else(|_| Vec::new());

        trace!(?path, "Opened index with {} entries", entries.len());

        Self { path, entries }
    }

    pub fn add(&mut self, path: &Path, oid: &Digest, stat: libc::stat) {
        trace!(?path, "Adding entry to index");
        let existing = self
            .entries
            .iter()
            .position(|e| e.name == path.as_os_str().as_bytes());

        if let Some(idx) = existing {
            //FIXE: maybe preserve order rather than just sorting later
            self.entries.swap_remove(idx);
        }

        let entry = IndexEntry::create(path, oid, stat).unwrap();
        //FIXE: maybe preserve order rather than just sorting later
        self.entries.push(entry);
        self.entries.sort_unstable();
    }

    pub fn write_out(&self) -> Result<()> {
        let index = Index::from_entries(&self.entries);
        let index = write::write_index(&index);
        std::fs::write(&self.path, index)?;
        Ok(())
    }

    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::{parse::*, write::*};

    #[test]
    #[ignore = "Doesn't work in CI"]
    fn read_write_index() {
        let bytes = std::fs::read("/home/jamie/Git/nixpkgs-official/.git/index").unwrap();
        let idx = parse_index(&bytes);

        for e in &idx.entries {
            let name = std::str::from_utf8(&e.name);
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
