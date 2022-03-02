use sha1::{Digest as _, Sha1};

use rand::prelude::*;

pub type Digest = [u8; 20];

pub fn hash(bytes: &[u8]) -> Digest {
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    let mut dig: Digest = Default::default();
    dig.copy_from_slice(&hasher.finalize()[..]);
    dig
}

pub fn tmp_file_name() -> String {
    const ALPHANUM_CHARS: [char; 52] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
        'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    let mut rng = rand::thread_rng();

    format!(
        "tmp_obj_{}{}{}{}{}{}",
        ALPHANUM_CHARS.choose(&mut rng).unwrap(),
        ALPHANUM_CHARS.choose(&mut rng).unwrap(),
        ALPHANUM_CHARS.choose(&mut rng).unwrap(),
        ALPHANUM_CHARS.choose(&mut rng).unwrap(),
        ALPHANUM_CHARS.choose(&mut rng).unwrap(),
        ALPHANUM_CHARS.choose(&mut rng).unwrap(),
    )
}
