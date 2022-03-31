use std::{
    fmt::{Debug, LowerHex},
    ops::{Deref, DerefMut},
};

use sha1::{Digest as _, Sha1};

#[derive(Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Digest(pub [u8; 20]);

impl Digest {
    pub fn new(bytes: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let mut dig: [u8; 20] = Default::default();
        dig.copy_from_slice(&hasher.finalize()[..]);
        Self(dig)
    }
    pub fn lower_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Deref for Digest {
    type Target = [u8; 20];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Digest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl LowerHex for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lower_hex())
    }
}

impl Debug for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Digest({})", self.lower_hex())
    }
}
