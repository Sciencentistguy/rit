use tracing::{trace, warn};

use super::{Index, IndexEntry, IndexHeader};
use crate::util::align_to;
use crate::Digest;

pub(super) fn parse_index(bytes: &[u8]) -> Index {
    trace!("Parsing bytes as index...");

    let header = parse_index_header(bytes[..12].try_into().unwrap());
    assert!(
        header.has_valid_magic(),
        "Read invalid header; {:?} != b\"DIRC\"",
        header.magic
    );

    assert_eq!(
        header.version, 2,
        "Only git index version 2 is supported (this is version {})",
        header.version
    );

    trace!("Successfully verified index header");

    let mut entries = Vec::new();
    let bytes = &bytes[12..];
    let mut offset = 0;
    trace!("Parsing index entries...");
    for _ in 0..header.num_entries {
        let entry = parse_index_entry(&mut offset, bytes);
        entries.push(entry);
    }
    trace!("Parsing index entries... done");

    if offset + 20 != bytes.len() - 1 {
        //TODO possibly support these
        warn!("This index has extensions. These are not supported yet and will be discarded");
    }

    let _oid = Digest(bytes[bytes.len() - 20..].try_into().unwrap());

    trace!("Parsing bytes as index... done");
    Index { header, entries }
}

fn parse_index_header(bytes: &[u8; 12]) -> IndexHeader {
    trace!("Parsing index header...");
    let magic = bytes[0..4].try_into().unwrap();
    let version = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    let num_entries = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
    trace!("Parsing index header... done");
    IndexHeader {
        magic,
        version,
        num_entries,
    }
}

fn parse_index_entry(offset: &mut usize, bytes: &[u8]) -> IndexEntry {
    let ctime_s = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    let ctime_n = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;

    let mtime_s = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    let mtime_n = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;

    let dev = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    let ino = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;

    let mode = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap()).into();
    *offset += 4;

    let uid = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    let gid = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    let siz = u32::from_be_bytes(bytes[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;

    // Digest
    let oid = Digest(bytes[*offset..*offset + 20].try_into().unwrap());
    *offset += 20;
    let flags = u16::from_be_bytes(bytes[*offset..*offset + 2].try_into().unwrap());
    *offset += 2;

    let name = {
        let len = unsafe { libc::strlen(bytes.as_ptr().add(*offset).cast()) };
        let slc = &bytes[*offset..*offset + len];
        *offset += len + 1;
        slc.to_owned()
    };

    // Pad the end of the name with NUL bytes to align the next entry to multiples of 8
    *offset = align_to(8, *offset);

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
    }
}
