use crate::storable::Storable;

use tracing::warn;


impl Storable for super::Commit {
    fn format(&self) -> Vec<u8> {
        let data = format!(
            "\
            tree {}\n\
            {}\
            author {} <{}> {}\n\
            committer {} <{}> {}\n\
            \n\
            {}",
            self.tree_id.to_hex(),
            if let Some(parent) = self.parents.first() {
                warn!("Only writing first parent, NYI");
                format!("parent {parent:x}\n")
            } else {
                "".into()
            },
            self.author.name,
            self.author.email,
            format!(
                "{} {}{:04}",
                self.author.when.unix,
                if self.author.when.offset.is_negative() {
                    '-'
                } else {
                    '+'
                },
                self.author.when.offset.abs()
            ),
            self.author.name,
            self.author.email,
            format!(
                "{} {}{:04}",
                self.committer.when.unix,
                if self.committer.when.offset.is_negative() {
                    '-'
                } else {
                    '+'
                },
                self.committer.when.offset.abs()
            ),
            self.message
        );
        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"commit ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(data.as_bytes());
        formatted
    }
}
