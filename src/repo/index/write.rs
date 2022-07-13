use tracing::warn;

use super::{Index, IndexEntry, IndexHeader};
use crate::Digest;

pub fn write_index(index: &Index) -> Vec<u8> {
    let mut siz = std::mem::size_of::<IndexHeader>();
    for entry in &index.entries {
        siz += std::mem::size_of::<IndexEntry>() + entry.name.to_bytes().len();
    }
    let mut out = Vec::new();
    write_index_header(&index.header, &mut out);

    for entry in &index.entries {
        write_index_entry(entry, &mut out);
    }

    let oid = Digest::new(&out);
    if oid != index.oid {
        warn!(
            "Writing index with differing oid: {:x} != {:x}",
            oid, index.oid
        );
    }

    out.extend_from_slice(&oid.0);

    out
}

fn write_index_header(hdr: &IndexHeader, dest: &mut Vec<u8>) {
    dest.extend_from_slice(&hdr.magic);
    dest.extend_from_slice(&hdr.version.to_be_bytes());
    dest.extend_from_slice(&hdr.num_entries.to_be_bytes());
}

fn write_index_entry(
    IndexEntry {
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
    }: &IndexEntry,
    out: &mut Vec<u8>,
) {
    let start_len = out.len();
    out.extend_from_slice(&ctime_s.to_be_bytes());
    out.extend_from_slice(&ctime_n.to_be_bytes());
    out.extend_from_slice(&mtime_s.to_be_bytes());
    out.extend_from_slice(&mtime_n.to_be_bytes());
    out.extend_from_slice(&dev.to_be_bytes());
    out.extend_from_slice(&ino.to_be_bytes());
    out.extend_from_slice(&mode.to_be_bytes());
    out.extend_from_slice(&uid.to_be_bytes());
    out.extend_from_slice(&gid.to_be_bytes());
    out.extend_from_slice(&siz.to_be_bytes());
    out.extend_from_slice(&oid.0);
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(name.to_bytes_with_nul());
    let len = out.len() - start_len;
    let extra = len % 8;
    if extra != 0 {
        let padsize = 8 - extra;
        for _ in 0..padsize {
            out.push(b'\0');
        }
    }
}
