use std::{
    fmt::{Debug, LowerHex},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use sha1::{Digest as _, Sha1};

#[derive(Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Digest(pub [u8; 20]);

impl Digest {
    /// Hash the input bytes and return the resulting digest.
    pub fn new(bytes: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let fin = hasher.finalize();
        debug_assert!(fin.len() == 20);
        // Unsafe dance to avoid writing 20 bytes of 0 and immediately overwriting it
        // Yes I know this is pointless over-optimisation but its my project so I'm allowed.
        unsafe {
            let mut buf: MaybeUninit<Self> = MaybeUninit::uninit();
            std::ptr::copy(fin.as_ptr(), buf.as_mut_ptr().cast(), 20);
            buf.assume_init()
        }
    }

    /// Format the digest as a hex string.
    ///
    /// Identical to `format!("{:x}", self)`.
    pub fn to_hex(&self) -> String {
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
        write!(f, "{}", self.to_hex())
    }
}

impl Debug for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Digest({})", self.to_hex())
    }
}

impl FromStr for Digest {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        assert!(bytes.len() == 20);
        Ok(Digest(bytes.try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1() {
        const HASH_INPUT: &[u8] = b"Hello, World!";

        // `printf 'Hello, World!' | sha1sum` => 0a0a9f2a6772942557ab5355d76af442f8f65e01
        const HASH_OUTPUT: [u8; 20] = [
            0x0a, 0x0a, 0x9f, 0x2a, 0x67, 0x72, 0x94, 0x25, 0x57, 0xab, 0x53, 0x55, 0xd7, 0x6a,
            0xf4, 0x42, 0xf8, 0xf6, 0x5e, 0x01,
        ];

        let actual = Digest::new(HASH_INPUT);
        println!("{}", actual.to_hex());
        assert_eq!(actual.0, HASH_OUTPUT);
    }
}
