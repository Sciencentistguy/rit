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
        let root = Tree::build(entries)?;
        trace!("Traversing root");
        root.traverse(|tree| self.database.store(&DatabaseObject::new(&*tree)))?;

        let root = DatabaseObject::new(&root);

        self.database.store(&root)?;

        let parent_commit = self.read_head()?;

        let name = std::env::var("RIT_AUTHOR_NAME")?;
        let email = std::env::var("RIT_AUTHOR_EMAIL")?;

        let commit = Commit::new(
            parent_commit,
            root.into_oid(),
            name,
            email,
            message.to_owned(),
        );

        let commit = DatabaseObject::new(&commit);

        self.database.store(&commit)?;
        self.set_head(commit.oid())?;

        Ok(commit.into_oid())
    }
}
