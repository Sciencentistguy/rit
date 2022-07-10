use std::path::Path;

use rand::prelude::*;

pub fn tmp_file_name() -> String {
    const ALPHANUM_CHARS: [char; 61] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
        'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '1', '2',
        '3', '4', '5', '6', '7', '8', '9',
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

pub trait Descends {
    fn descends(&self) -> Vec<&Path>;
}

impl Descends for Path {
    fn descends(&self) -> Vec<&Path> {
        let mut descends: Vec<_> = self.ancestors().collect();
        let _ = descends.pop();
        descends.reverse();
        descends
    }
}
