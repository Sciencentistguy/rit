pub mod blob;
pub mod commit;
pub mod tree;

use crate::digest::Digest;

pub trait Storable {
    /// Returns the storable uncompressed but formatted `"{type} {len}\0{data}"`.
    /// e.g. a blob `"hello\n"` becomes `"blob 6\0hello\n"`
    fn formatted(&self) -> &[u8];
    fn oid(&self) -> &Digest;
    fn into_oid(self) -> Digest;
}
