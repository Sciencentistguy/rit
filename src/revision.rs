//! A revision is valid if it matches the following (informally defined) context-free grammar:
//! `<rev>` = `<refname>`
//! `<rev>` = `<rev>^`
//! `<rev>` = `<rev>~<num>`
//! `<num>` = a natural number
//! `<refname>` = <branchname> | <sha1> | HEAD

use std::str::FromStr;

use crate::{digest::Digest, Result};

use color_eyre::eyre::{eyre, Context};
/// Contains all characters that cannot appear in a ref name.
///
/// In git, the character `'*'` is allowed in ref names if the environment variable
/// `REFNAME_REFSPEC_PATTERN` is set. Rit does not allow this, so `'*'` appears in this array.
///
/// Also, Git uses C-Strings; the character `'\0'` denotes the end of a ref name ref nams. We
/// disallow it entirely.
///
/// See: <https://github.com/git/git/blob/795ea8776befc95ea2becd8020c7a284677b4161/refs.c#L48-L57>
const DISALLOWED_CHARACTERS: [char; 41] = [
    '\0', '\x01', '\x02', '\x03', '\x04', '\x05', '\x06', '\x07', '\x08', '\t', '\n', '\x0b',
    '\x0c', '\r', '\x0e', '\x0f', '\x10', '\x11', '\x12', '\x13', '\x14', '\x15', '\x16', '\x17',
    '\x18', '\x19', '\x1a', '\x1b', '\x1c', '\x1d', '\x1e', '\x1f', ' ', '*', ':', '?', '[', '\\',
    '^', '~', '\x7f',
];

/// Check whether a string is a valid ref name.
///
/// This is not a port of `check_refname_component` from git's `refs.c`, but is based on the documentation for
/// that function.
///
/// Disallowed paths are any path where:
///
/// - it (or any path component) begins with `'.'`
/// - it contains double dots `".."`
/// - it contains ASCII control characters
/// - it contains ':', '?', '[', '\', '^', '~', SP, or TAB anywhere
/// - it contains `'*'`
/// - it ends with `'/'`
/// - it ends with `".lock"`
/// - it contains `"@{"`
///
/// See: <https://github.com/git/git/blob/795ea8776befc95ea2becd8020c7a284677b4161/refs.c#L59-L77>
pub fn is_valid_ref_name(name: &str) -> bool {
    !((name.chars().any(|c| DISALLOWED_CHARACTERS.contains(&c)))
        || name.starts_with('.')
        || name.contains("/.")
        || name.contains("..")
        || name.ends_with('/')
        || name.ends_with(".lock")
        || name.contains("@{"))
}

#[derive(Debug, PartialEq, Eq)]
enum Rev {
    Refname(Refname),
    Parent(Box<Rev>),
    Ancestor(Box<Rev>, u64),
}

impl Rev {
    fn parse(input: &str) -> Result<Self> {
        if let Some(rev) = input.strip_suffix('^') {
            let rev = Self::parse(rev)?;
            return Ok(Rev::Parent(Box::new(rev)));
        }

        if let Some(idx) = input.rfind('~') {
            let distance = &input[idx + 1..];
            let distance = distance
                .parse::<u64>()
                .wrap_err(eyre!("A number is required after '~'"))?;

            let rev = Self::parse(&input[..idx])?;
            return Ok(Rev::Ancestor(Box::new(rev), distance));
        }

        // else...
        Ok(Rev::Refname(Refname::parse(input)?))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Refname {
    BranchTag(String),
    Sha1(Digest),
    Head,
}

impl Refname {
    fn parse(input: &str) -> Result<Self> {
        if matches!(input, "HEAD" | "@") {
            return Ok(Self::Head);
        }

        if let Ok(digest) = Digest::from_str(input) {
            return Ok(Self::Sha1(digest));
        }

        if !is_valid_ref_name(input) {
            return Err(eyre!("Invalid ref name: {}", input));
        }

        Ok(Self::BranchTag(input.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head() {
        let rev = "HEAD";
        let rev = Rev::parse(rev).unwrap();
        assert_eq!(rev, Rev::Refname(Refname::Head));
    }

    #[test]
    fn sha1() {
        let rev = "ffc1c862714edb677d6f467902cf2e406eee22ce";
        let rev = Rev::parse(rev).unwrap();
        let dig = Digest::from_str("ffc1c862714edb677d6f467902cf2e406eee22ce").unwrap();
        assert_eq!(rev, Rev::Refname(Refname::Sha1(dig)));
    }

    #[test]
    fn branch_tag() {
        let branch_tag = ["master", "main", "origin/main", "v1.0.0"];
        for name in branch_tag {
            dbg!(name);
            let rev = Rev::parse(name).unwrap();
            assert_eq!(rev, Rev::Refname(Refname::BranchTag(name.to_owned())));
        }
    }

    #[test]
    fn parents() {
        let parents = ["HEAD^", "HEAD^^"];

        for rev in parents {
            dbg!(&rev);
            let rev = Rev::parse(rev).unwrap();
            assert!(matches!(rev, Rev::Parent(_)));
        }

        let rev = Rev::parse(parents[1]).unwrap();
        assert_eq!(
            rev,
            Rev::Parent(Box::new(Rev::Parent(Box::new(Rev::Refname(Refname::Head))))),
        );
    }

    #[test]
    fn ancestors() {
        let ancestors = ["HEAD~1", "HEAD~2", "HEAD~3", "HEAD~1012123119"];
        for rev in ancestors {
            dbg!(rev);
            let rev = Rev::parse(rev).unwrap();
            assert!(matches!(rev, Rev::Ancestor(_, _)));
        }
    }

    #[test]
    fn complex() {
        let complex = "HEAD~12^^~2";
        let rev = Rev::parse(complex).unwrap();
        assert_eq!(
            rev,
            Rev::Ancestor(
                Box::new(Rev::Parent(Box::new(Rev::Parent(Box::new(Rev::Ancestor(
                    Box::new(Rev::Refname(Refname::Head)),
                    12,
                )))))),
                2
            )
        );
    }

    #[test]
    fn invalid() {
        let invalid = [
            "HEAD~",
            "HEAD~-1",
            "HEAD~^",
            "mast\0er",
            "HEAD^2",
            "HEAD~99999999999999999999999999999999999999999999999999",
        ];

        for rev in invalid {
            dbg!(rev);
            assert!(Rev::parse(rev).is_err());
        }
    }
    #[test]
    fn book() {
        let book_testcases = ["@^", "HEAD~42", "master^^", "abc123~3"];

        for rev in book_testcases {
            dbg!(rev);
            assert!(Rev::parse(rev).is_ok());
        }
    }
}
