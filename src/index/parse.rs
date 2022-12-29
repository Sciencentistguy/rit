use tracing::trace;

use super::{Index, IndexEntry, IndexHeader};
use crate::Result;

pub(super) fn parse_index(bytes: &[u8]) -> Result<Index> {
    trace!("Parsing bytes as index...");
    let (_, index) = nom::parse_index(bytes).unwrap();
    index
}

mod nom {
    use nom::{
        bytes::complete::{tag, take, take_till},
        number::complete::{be_u16, be_u32},
    };

    use crate::{digest::Digest, filemode::FileMode, index::IndexHeader};

    pub type Input<'a> = &'a [u8];
    pub type Result<'a, O> = nom::IResult<Input<'a>, O, nom::error::VerboseError<Input<'a>>>;
    pub type BitResult<'a, O> = nom::IResult<Input<'a>, O>;

    pub(super) fn parse_index(i: Input) -> Result<crate::Result<super::Index>> {
        let (mut i, header) = parse_index_header(i)?;

        let mut entries = Vec::with_capacity(header.num_entries as usize);
        for _ in 0..header.num_entries {
            let (new_i, entry) = parse_index_entry(i)?;
            match entry {
                Ok(entry) => {
                    i = new_i;
                    entries.push(entry);
                }
                Err(e) => return Ok((i, Err(e))),
            }
        }

        Ok((i, Ok(super::Index { header, entries })))
    }

    fn parse_index_header(i: Input) -> Result<super::IndexHeader> {
        let (i, magic) = tag(IndexHeader::MAGIC)(i)?;
        let (i, version) = be_u32(i)?;
        let (i, num_entries) = be_u32(i)?;
        Ok((
            i,
            super::IndexHeader {
                magic: magic.try_into().unwrap(),
                version,
                num_entries,
            },
        ))
    }

    fn parse_index_entry(i: Input) -> Result<crate::Result<super::IndexEntry>> {
        let (i, ctime_s) = be_u32(i)?;
        let (i, ctime_n) = be_u32(i)?;
        let (i, mtime_s) = be_u32(i)?;
        let (i, mtime_n) = be_u32(i)?;
        let (i, dev) = be_u32(i)?;
        let (i, ino) = be_u32(i)?;
        let (i, mode) = be_u32(i)?;
        let (i, uid) = be_u32(i)?;
        let (i, gid) = be_u32(i)?;
        let (i, siz) = be_u32(i)?;
        let (i, oid) = take(20usize)(i)?;
        let (i, flags) = be_u16(i).map(|(i, x)| (i, IndexEntryFlags(x)))?;
        // let (i, flags) = be_u16(i)?;

        // dbg!(flags.name_length());
        let (i, name) = if let Some(len) = flags.name_length() {
            take(len)(i)?
        } else {
            take_till(|x| x == b'\0')(i)?
        };
        dbg!(&i[..16]);
        // let (i, name) = take_till(|x| x == b'\0')(i)?;
        // let len = name.len();
        // let padded_len = crate::util::align_to(8, len);
        // let (i, _padding) = take(padded_len - len)(i)?;
        // dbg!(_padding);

        let (i, _padding) = nom::bytes::complete::take_while(|x| x == b'\0')(i)?;

        let mode = FileMode::from(mode);
        let oid = Digest(oid.try_into().unwrap());
        let name = match String::from_utf8(name.to_owned()) {
            Ok(x) => x,
            Err(e) => return Ok((i, Err(e.into()))),
        };

        dbg!(&name);

        Ok((
            i,
            Ok(dbg!(super::IndexEntry {
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
                flags: flags.0,
                name,
            })),
        ))
    }

    #[repr(transparent)]
    struct IndexEntryFlags(u16);

    impl IndexEntryFlags {
        fn name_length(&self) -> Option<usize> {
            let len = self.0 & 0xFFF;
            if len == 0xFFF {
                None
            } else {
                Some(len as usize)
            }
        }
    }
}
