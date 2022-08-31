use std::{
    fmt::{Debug, LowerHex},
    ops::{Deref, DerefMut},
    str::FromStr,
};

use hex::FromHexError;
use sha1::{Digest as _, Sha1};
use tap::Tap;

#[derive(Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Digest(pub [u8; 20]);

impl Digest {
    /// The null digest, 0x00000...
    ///
    /// This is used for deleted / missing files.
    pub const NULL: Self = Digest([0; 20]);
}

impl Digest {
    /// Hash the input bytes and return the resulting digest.
    pub fn new(bytes: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let fin = hasher.finalize();
        assert_eq!(fin.len(), 20);
        // Copy 20 bytes out of the GenericArray and transmute to `Self`
        //
        // This is safe because:
        // - `GenericArray` is marked as `#[repr(transparent)]`
        // - `fin.len()` is `20`
        // - `Self` is marked as `#[repr(transparent)]`, and `self.0.len()` os `20`
        unsafe { fin.as_ptr().cast::<Self>().read() }
    }

    /// Format the digest as a hex string.
    ///
    /// Identical to `format!("{:x}", self)`.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Shorten a Digest, usually for display purposes.
    ///
    /// Note: This doesn't check for collisions.
    pub fn short(&self) -> String {
        self.to_hex().tap_mut(|x| x.truncate(7))
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
        if bytes.len() != 20 {
            return Err(FromHexError::InvalidStringLength);
        }
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

    #[test]
    fn test_from_str() {
        let valid = [
            "0a0a9f2a6772942557ab5355d76af442f8f65e01",
            "0A0A9F2A6772942557AB5355D76AF442F8F65E01",
            "0a0a9f2a6772942557ab5355D76AF442F8F65E01",
        ];

        for string in valid {
            let _ = Digest::from_str(string).unwrap();
        }

        let invalid = [
            "hello world",
            "0j0a9f2a6772942557ab5355d76af442f8f65e01",
            "🦀",
            "0a0a9f2a6772942557ab5355d76af442f8f65e01 ",
            " 0a0a9f2a6772942557ab5355d76af442f8f65e01",
            "0a0a9f2a6772942557ab5355d76af442f8f65e01\n",
            "0a0a9f2a6772942557ab5355d76af442f8f65e01\0",
            "0a0a9f2a6772942\x0057ab5355d76af442f8f65e01",
            "",
        ];

        for string in invalid {
            let _ = Digest::from_str(string).unwrap_err();
        }
    }
}
