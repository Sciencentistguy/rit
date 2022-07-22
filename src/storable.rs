use crate::digest::Digest;

pub trait Storable {
    /// Format `self.inner` uncompressed:
    ///
    /// ## Example
    /// A blob with contents `"hello\n"` becomes `"blob 6\0hello\n"`
    fn format(&self) -> Vec<u8>;

    fn oid(&self, formatted: &[u8]) -> Digest {
        Digest::new(formatted)
    }
}

/// A wrapper for a type that can be stored in the database.
///
/// Constructing this type ensures that the data is formatted correctly for storage, and has an oid
pub struct DatabaseObject<'a, T>
where
    T: Storable,
{
    pub inner: &'a T,
    formatted: Vec<u8>,
    oid: Digest,
}

impl<'a, T: Storable> DatabaseObject<'a, T> {
    pub fn new(inner: &'a T) -> Self {
        let formatted = inner.format();
        let oid = inner.oid(&formatted);

        Self {
            inner,
            formatted,
            oid,
        }
    }

    pub fn formatted(&self) -> &[u8] {
        self.formatted.as_ref()
    }

    pub fn oid(&self) -> &Digest {
        &self.oid
    }

    pub fn into_oid(self) -> Digest {
        self.oid
    }
}
