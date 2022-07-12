use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::CStr,
    os::unix::prelude::OsStrExt,
    path::{Path, PathBuf},
};

use crate::digest::Digest;
use crate::Result;

// pub struct Index {
// index_path: PathBuf,
// entries: HashMap<String, IndexEntry>,
// }

// struct BinIndex {
// header: IndexHeader,
// entries: [IndexEntry],
// }

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

    // fn create(path: &Path, oid: Digest, stat: libc::stat) -> Result<Self> {
    // let name = path
    // .file_name()
    // .expect("File should have name")
    // .as_bytes()
    // .to_owned();

    // let flags = path
    // .as_os_str()
    // .len()
    // .try_into()
    // .unwrap_or(Self::MAX_PATH_SIZE);

    // Ok(Self {
    // ctime_s: stat.st_ctime.try_into()?,
    // ctime_n: stat.st_ctime_nsec.try_into()?,
    // mtime_s: stat.st_mtime.try_into()?,
    // mtime_n: stat.st_mtime_nsec.try_into()?,
    // dev: stat.st_dev.try_into()?,
    // ino: stat.st_ino.try_into()?,
    // mode: stat.st_mode,
    // uid: stat.st_uid,
    // gid: stat.st_gid,
    // siz: stat.st_size.try_into()?,
    // oid,
    // flags,
    // name,
    // })
    // }
}

// impl Index {
// pub fn new(path: &Path) -> Self {
// Self {
// index_path: path.join(".git/index"),
// entries: HashMap::new(),
// }
// }

// pub fn add(&mut self, pathname: &Path, oid: Digest, stat: libc::stat) -> Result<()> {
// let entry = IndexEntry::create(pathname, oid, stat);
// self.entries
// .insert(pathname.to_str().expect("utf-8").to_owned(), entry?);
// Ok(())
// }
// }

struct Index<'a> {
    header: IndexHeader,
    entries: Vec<IndexEntry<'a>>,
}

fn parse_index(bytes: &[u8]) -> Index {
    let header = parse_index_header(bytes[..12].try_into().unwrap());
    assert!(
        header.has_valid_magic(),
        "Read invalid header; {:?} != b\"DIRC\"",
        header.magic
    );

    let mut entries = Vec::new();
    let mut offset = 12;
    for _ in 0..header.num_entries {
        let ctime_s = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let ctime_n = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let mtime_s = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let mtime_n = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let dev = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let ino = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let mode = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let uid = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let gid = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let siz = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        // Digest
        let oid = Digest(bytes[offset..offset + 20].try_into().unwrap());
        offset += 20;
        let flags = u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let name = {
            let len = unsafe { libc::strlen(bytes.as_ptr().add(offset).cast()) } + 1;
            let slc = &bytes[offset..offset + len];
            offset += len + (len % 8);
            CStr::from_bytes_with_nul(slc).unwrap()
        };
        entries.push(IndexEntry {
            ctime_s,
            ctime_n,
            mtime_s,
            mtime_n,
            dev,
            ino,
            mode,
            uid,
            gid,
            siz,
            oid,
            flags,
            name,
        });
    }

    Index { header, entries }
}

fn parse_index_header(bytes: &[u8; 12]) -> IndexHeader {
    let magic = bytes[0..4].try_into().unwrap();
    let version = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    let num_entries = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
    IndexHeader {
        magic,
        version,
        num_entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works() {
        let bytes = std::fs::read("/dev/shm/rit/.git/index").unwrap();
        let idx = parse_index(&bytes);
        println!("{:?}", idx.entries[0].ctime_s);
        println!("{:?}", idx.entries[0].name);
        let s = idx.entries[1].name.to_str().unwrap();
        println!("{} ({:?})", s, idx.entries[1].name);
    }
}
