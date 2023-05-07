mod parse;
mod write;

use std::fmt::Display;

use chrono::NaiveDateTime;

use crate::digest::Digest;
use crate::repo::Repo;
use crate::timestamp::Timestamp;
use crate::Result;

struct GpgSig;

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
        self.committer.when.0.date_naive()
    }
}
