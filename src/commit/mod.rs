mod parse;
mod write;

use std::fmt::Display;

use chrono::{NaiveDate, NaiveDateTime};

use crate::digest::Digest;
use crate::repo::Repo;
use crate::Result;

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
        self.to_string()
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
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

    pub fn parents(&self) -> &[Digest] {
        self.parents.as_ref()
    }

    pub fn parent(&self, repo: &Repo) -> Result<Option<Commit>> {
        let parent = match self.parents.first() {
            Some(x) => x,
            None => return Ok(None),
        };
        let parent = repo
            .database
            .load(parent)?
            .into_commit()
            .expect("The parent of a commit should be a commit");

        Ok(Some(parent))
    }

    pub fn message(&self) -> &str {
        self.message.as_ref()
    }

    pub fn pretty_print(&self) -> std::io::Result<()> {
        println!("tree {:x}", self.tree_id);
        for parent in &self.parents {
            println!("parent {parent:x}");
        }
        println!(
            "author {} <{}> {}",
            self.author.name, self.author.email, self.author.when
        );
        println!(
            "committer {} <{}> {}",
            self.committer.name, self.committer.email, self.committer.when
        );

        if let Some(_gpgsig) = &self.gpgsig {
            println!("gpgsig [not yet implemented]");
        }
        println!();

        println!("{}", self.message);

        Ok(())
    }

    pub(crate) fn commit_date(&self) -> chrono::NaiveDate {
        let unix = self.committer.when.unix;
        NaiveDateTime::from_timestamp(
            unix.try_into()
                // If you're somehow using this crate in 300 billion years time, where unix
                // timestamps don't fit in an i64, then I'm sorry.
                .expect("Timestamp should be positive and fit in an i64."),
            0,
        )
        .date()
    }
}
