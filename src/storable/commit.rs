use crate::digest::Digest;

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
    pub fn new(parent_commit: Option<Digest>, tree: Digest, author: Author, message: &str) -> Self {
        let parent = parent_commit.map(|x| x.to_hex());

        let data = format!(
            "\
            tree {}\n\
            {}\
            author {} <{}> {}\n\
            committer {} <{}> {}\n\
            \n\
            {}",
            tree.to_hex(),
            if let Some(parent) = parent {
                format!("parent {parent}\n")
            } else {
                "".into()
            },
            author.name,
            author.email,
            chrono::offset::Local::now().format("%s %z"),
            author.name,
            author.email,
            chrono::offset::Local::now().format("%s %z"),
            message
        );

        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"commit ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(data.as_bytes());

        let oid = Digest::new(&formatted);

        Self { formatted, oid }
    }
}

impl Storable for Commit {
    fn formatted(&self) -> &[u8] {
        &self.formatted
    }

    fn oid(&self) -> &Digest {
        &self.oid
    }

    fn into_oid(self) -> Digest {
        self.oid
    }
}
