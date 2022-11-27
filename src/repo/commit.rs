use tracing::trace;

use crate::commit::Commit;
use crate::digest::Digest;
use crate::storable::DatabaseObject;
use crate::tree::Tree;
use crate::Result;

impl super::Repo {
    pub fn commit(&mut self, message: &str) -> Result<Digest> {
        trace!(path=?self.dir, %message, "Starting commit");
        let entries = &self.index.entries();
        let root = Tree::build(entries).unwrap();
        trace!("Traversing root");
        root.traverse(|tree| self.database.store(&DatabaseObject::new(&*tree))).unwrap();

        let root = DatabaseObject::new(&root);

        self.database.store(&root).unwrap();

        let parent_commit = self.read_head().unwrap();

        let name = std::env::var("RIT_AUTHOR_NAME").unwrap();
        let email = std::env::var("RIT_AUTHOR_EMAIL").unwrap();

        let commit = Commit::new(
            parent_commit,
            root.into_oid(),
            name,
            email,
            message.to_owned(),
        );

        let commit = DatabaseObject::new(&commit);

        self.database.store(&commit).unwrap();
        self.set_head(commit.oid()).unwrap();

        Ok(commit.into_oid())
    }
}
