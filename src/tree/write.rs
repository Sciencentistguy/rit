use crate::{digest::Digest, storable::Storable};

use super::TreeEntry;

impl Storable for super::Tree {
    fn format(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for (name, entry) in self.entries.iter() {
            data.extend_from_slice(format!("{:o}", entry.mode()).as_bytes());
            data.push(b' ');
            data.extend_from_slice(name.as_bytes());
            data.push(b'\0');
            if let TreeEntry::Directory { tree, .. } = entry {
                let _ = tree.format(); // force the subtree to caluclate all its oids
            }
            let oid = entry.oid().expect("subtree oid should have been inited");
            data.extend_from_slice(&**oid);
        }

        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"tree ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(&data);

        let oid = Digest::new(&formatted);

        match self.oid.set(oid) {
            Ok(_) => {}
            Err(oid) => {
                debug_assert_eq!(
                    oid,
                    self.oid.get().cloned().unwrap(),
                    "Oid should not change during formatting"
                );
            }
        }

        formatted
    }

    fn oid(&self, _: &[u8]) -> Digest {
        self.oid
            .get()
            .cloned()
            .expect("oid should have been inited")
    }
}
