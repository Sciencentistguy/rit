use crate::util::Digest;
use crate::util;

use once_cell::sync::OnceCell;

pub trait Storable {
    fn format(&self) -> Vec<u8>;
    fn get_oid(&self) -> &Digest;
}


pub struct Blob {
    oid: OnceCell<Digest>,
    pub data: Vec<u8>,
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            oid: Default::default(),
            data,
        }
    }
}

impl Storable for Blob {
    fn format(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"blob ");
        out.extend_from_slice(format!("{}", self.data.len()).as_bytes());
        out.push(b'\0');
        out.extend_from_slice(&self.data);
        self.oid.get_or_init(|| util::hash(&out));
        out
    }

    fn get_oid(&self) -> &Digest {
        self.oid.get().expect("OID not yet set")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_format() {
        let text = "hello\n";
        let expected_hash = &[
            206, 1, 54, 37, 3, 11, 168, 219, 169, 6, 247, 86, 150, 127, 158, 156, 163, 148, 70, 74,
        ];
        let blob = Blob::new(text.as_bytes().to_owned());
        assert_eq!(blob.oid.get(), None);
        let formatted = blob.format();
        assert_eq!(formatted, b"blob 6\0hello\n");
        assert_eq!(blob.oid.get(), Some(expected_hash));
        let formatted_again = blob.format();
        assert_eq!(formatted_again, b"blob 6\0hello\n");
        assert_eq!(blob.oid.get(), Some(expected_hash));
    }
}
