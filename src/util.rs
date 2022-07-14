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

pub fn align_to(n: usize, num: usize) -> usize {
    let extra = num % n;
    match extra {
        0 => num,
        extra => {
            let padsize = n - extra;
            num + padsize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that align_to works as expected for n=8 (the only one used at the time of writing)
    fn test_align_to_8() {
        for i in 1..9 {
            assert_eq!(align_to(8, i), 8);
        }
        assert!((0..64).map(|i| align_to(8, i)).all(|x| x % 8 == 0));
    }
}
