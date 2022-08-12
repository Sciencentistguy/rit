mod parse;
mod write;

use crate::digest::Digest;

struct GpgSig;

#[derive(PartialEq, Eq, Debug, Clone)]
struct Timestamp {
    unix: u64,
    offset: i64,
}

impl Timestamp {
    pub fn now() -> Self {
        let unix = chrono::offset::Local::now()
            .timestamp()
            .try_into()
            .expect("Time should be positive");
        let offset_seconds = chrono::offset::Local::now().offset().utc_minus_local() as i64;
        let offset_hours = offset_seconds / 60;
        let offset = offset_hours * 100;

        Self { unix, offset }
    }

    fn format(&self) -> String {
        format!(
            "{} {}{:04}",
            self.unix,
            if self.offset.is_negative() { '-' } else { '+' },
            self.offset.abs()
        )
    }
}

#[derive(Debug, Clone)]
struct Signature {
    name: String,
    email: String,
    when: Timestamp,
}

pub struct Commit {
    tree_id: Digest,
    parents: Vec<Digest>,
    author: Signature,
    committer: Signature,
    gpgsig: Option<GpgSig>,
    message: String,
}

impl Commit {
    pub fn new(
        parent_commit: Option<Digest>,
        tree_id: Digest,
        name: String,
        email: String,
        message: String,
    ) -> Self {
        let author = Signature {
            name,
            email,
            when: Timestamp::now(),
        };
        let committer = author.clone();

        Commit {
            tree_id,
            parents: parent_commit.into_iter().collect::<Vec<_>>(),
            author,
            committer,
            gpgsig: None,
            message,
        }
    }

    pub fn tree_id(&self) -> &Digest {
        &self.tree_id
    }
}
