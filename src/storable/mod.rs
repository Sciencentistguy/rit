pub mod blob;
pub mod commit;
pub mod tree;

use crate::util::Digest;

pub trait Storable {
    /// Returns the storable uncompressed but formatted `"{type} {len}\0{data}"`.
    /// e.g.
    /// a blob `"hello\n"` becomes `"blob 6\0hello\n"`
    fn format(&self) -> &[u8];
    fn get_oid(&self) -> &Digest;
    fn into_oid(self) -> Digest;
}
