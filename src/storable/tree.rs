use std::{ffi::OsStr, fs::Metadata, os::unix::prelude::*};

use crate::digest::Digest;

use super::Storable;

pub struct Entry {
    filename: Vec<u8>,
    oid: Digest,
    mode: u32,
}

impl Entry {
    pub fn new(filename: &OsStr, oid: Digest, metadata: Metadata) -> Self {
        Self {
            filename: filename.as_bytes().to_owned(),
            oid,
            mode: metadata.mode(),
        }
    }
}

pub struct Tree {
    formatted: Vec<u8>,
    oid: Digest,
}

impl Tree {
    pub fn new(mut entries: Vec<Entry>) -> Self {
        entries.sort_unstable_by(|a, b| a.filename.cmp(&b.filename));

        let mut data = Vec::new();
        for entry in &entries {
            let mode = format!("{:o}", entry.mode);
            data.extend_from_slice(mode.as_bytes());
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
