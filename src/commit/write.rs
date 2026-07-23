use std::fmt::Write;

use crate::storable::Storable;

// NOTE: non-utf8 in commit messages or author names will destroy the world
impl Storable for super::Commit {
    fn format(&self) -> Vec<u8> {
        let mut data = format!("tree {}\n", self.tree_id.to_hex());

        for parent in &self.parents {
            writeln!(&mut data, "parent {parent:x}").unwrap();
        }

        write!(
            &mut data,
            "\
            author {} <{}> {}\n\
            committer {} <{}> {}\n\
            \n\
            {}",
            self.author.name,
            self.author.email,
            self.author.when,
            self.committer.name,
            self.committer.email,
            self.committer.when,
            self.message
        )
        .unwrap();

        if !data.ends_with('\n') {
            data.push('\n');
        }

        let prefix = format!("commit {}\0", data.len());
        data.insert_str(0, &prefix);

        data.into_bytes()
    }
}
