use hex::ToHex;

use crate::util::{Digest, self};

use super::Storable;

pub struct Author {
    pub name: String,
    pub email: String,
}

pub struct Commit {
    formatted: Vec<u8>,
    oid: Digest,
}

impl Commit {
    pub fn new(tree_oid: Digest, author: Author, message: &str) -> Self {
        let l1 = format!("tree {}", tree_oid.encode_hex::<String>());
        let l2 = format!(
            "author {} <{}> {}",
            author.name,
            author.email,
            chrono::offset::Local::now().format("%s %z"),
        );
        let l3 = format!(
            "commiter {} <{}> {}",
            author.name,
            author.email,
            chrono::offset::Local::now().format("%s %z"),
        );
        let data = format!("{}\n{}\n{}\n\n{}\n", l1, l2, l3, message);

        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"commit ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(data.as_bytes());

        let oid = util::hash(&formatted);

        Self {
            formatted, 
            oid
        }
    }
}

impl Storable for Commit {
    fn format(&self) -> &[u8] {
        &self.formatted
    }

    fn get_oid(&self) -> &Digest {
        &self.oid
    }

    fn into_oid(self) -> Digest {
        self.oid
    }
}
