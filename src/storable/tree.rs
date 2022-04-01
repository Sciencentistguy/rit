use std::{ffi::OsStr, os::unix::prelude::OsStrExt};

use crate::digest::Digest;

use super::Storable;

pub struct Entry {
    filename: Vec<u8>,
    oid: Digest,
}

impl Entry {
    pub fn new(filename: &OsStr, oid: Digest) -> Self {
        Self {
            filename: filename.as_bytes().to_owned(),
            oid,
        }
    }
}

pub struct Tree {
    formatted: Vec<u8>,
    oid: Digest,
}

impl Tree {
    pub fn new(mut entries: Vec<Entry>) -> Self {
        const MODE: &[u8] = b"100644";
        entries.sort_unstable_by(|a, b| a.filename.cmp(&b.filename));

        let mut data = Vec::new();
        for entry in &entries {
            data.extend_from_slice(MODE);
            data.push(b' ');
            data.extend_from_slice(&entry.filename);
            data.push(b'\0');
            data.extend_from_slice(&*entry.oid);
        }

        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"tree ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(&data);
        let oid = Digest::new(&formatted);

        Self { formatted, oid }
    }
}

impl Storable for Tree {
    fn formatted(&self) -> &[u8] {
        // "{type} {len}\0{formatted}"
        &self.formatted
    }

    fn get_oid(&self) -> &Digest {
        &self.oid
    }

    fn into_oid(self) -> Digest {
        self.oid
    }
}
