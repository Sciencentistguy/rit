use std::io::Write;

use crate::storable::Storable;

pub struct Blob {
    data: Vec<u8>,
}

impl Storable for Blob {
    fn format(&self) -> Vec<u8> {
        let mut formatted = Vec::new();
        formatted.extend_from_slice(b"blob ");
        formatted.extend_from_slice(format!("{}", self.data.len()).as_bytes());
        formatted.push(b'\0');
        formatted.extend_from_slice(&self.data);
        formatted
    }
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// Pretty-printing a blob is simple - just dump the contents of the file to stdout
    pub fn pretty_print(&self) -> std::io::Result<()> {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(&self.data)?;
        stdout.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{digest::Digest, storable::DatabaseObject};

    use super::*;

    #[test]
    /// Generate a blob with known contents. Ensure that the OID and the formatted output are as
    /// expected.
    fn test_blob_format() {
        let text = b"hello\n";
        let expected_hash = Digest([
            206, 1, 54, 37, 3, 11, 168, 219, 169, 6, 247, 86, 150, 127, 158, 156, 163, 148, 70, 74,
        ]);
        let blob = Blob::new(text.to_vec());
        let blob = DatabaseObject::new(&blob);
        let formatted = blob.formatted();
        assert_eq!(*blob.oid(), expected_hash);
        assert_eq!(formatted, b"blob 6\0hello\n");
    }
}
