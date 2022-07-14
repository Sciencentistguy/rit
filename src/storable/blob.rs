use super::Storable;

use crate::digest::Digest;

pub struct Blob {
    oid: Digest,
    formatted: Vec<u8>,
}

impl Storable for Blob {
    fn formatted(&self) -> &[u8] {
        &self.formatted
    }

    fn get_oid(&self) -> &Digest {
        &self.oid
    }

    fn into_oid(self) -> Digest {
        self.oid
    }
}

impl Blob {
    pub fn new(data: &[u8]) -> Self {
        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"blob ");
        formatted.extend_from_slice(format!("{}", data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(data);
        let oid = Digest::new(&formatted);

        Self { oid, formatted }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Generate a blob with known contents. Ensure that the OID and the formatted output are as
    /// expected.
    fn test_blob_format() {
        let text = "hello\n";
        let expected_hash = [
            206, 1, 54, 37, 3, 11, 168, 219, 169, 6, 247, 86, 150, 127, 158, 156, 163, 148, 70, 74,
        ];
        let blob = Blob::new(text.as_bytes());
        let formatted = blob.formatted();
        assert_eq!(*blob.oid, expected_hash);
        assert_eq!(formatted, b"blob 6\0hello\n");
    }
}
