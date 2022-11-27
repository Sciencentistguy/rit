//! A revision is valid if it matches the following (informally defined) context-free grammar:
//! `<rev>` = `<refname>`
//! `<rev>` = `<rev>^`
//! `<rev>` = `<rev>~<num>`
//! `<num>` = a natural number
//! `<refname>` = a branch name | a sha1 hash | "HEAD" or '@'

use std::str::FromStr;

use crate::{
    digest::Digest,
    repo::{database::LoadedItem, Repo},
    Result,
};

use color_eyre::eyre::{eyre, Context};

/// Contains all characters that cannot appear in a ref name.
///
/// In git, the character `'*'` is allowed in ref names if the environment variable
/// `REFNAME_REFSPEC_PATTERN` is set. This feature is currently unsupported, and such `'*'` is a
/// disallowed character.
///
/// Also, Git uses C-Strings; the character `'\0'` denotes the end of a ref name. We
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
/// Disallowed paths are any path where:
/// - it (or any path component) begins with `'.'`
/// - it contains double dots `".."`
/// - it contains ASCII control characters
/// - it contains `':'`, `'?'`, `'['`, `'\\'`, `'^'`, `'~'`, `' '`, or `'\t'`
/// - it contains `'*'` (unless `REFNAME_REFSPEC_PATTERN` is set) [unsupported]
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
pub struct Rev {
    refname: Refname,
    distance: u64,
}

impl Rev {
    pub fn parse(mut input: &str) -> Result<Self> {
        let mut distance = 0;
        loop {
            if input.ends_with('^') {
                distance += 1;
                input = &input[..input.len() - 1];
            } else if let Some(idx) = input.rfind('~') {
                distance += input[idx + 1..]
                    .parse::<u64>()
                    .wrap_err(eyre!("A number is required after '~'"))?;
                input = &input[..idx];
            } else {
                let refname = Refname::parse(input)?;
                break Ok(Rev { refname, distance });
            }
        }
    }

    pub fn resolve(self, repo: &Repo) -> Result<Option<Digest>> {
        let Self {
            refname,
            mut distance,
        } = self;

        if distance == 0 {
            return refname.resolve(repo);
        }

        let intermediary = match refname.resolve(repo)? {
            Some(x) => x,
            None => return Ok(None),
        };

        let mut commit = repo
            .database
            .load(&intermediary)?
            .into_commit()
            .ok_or_else(|| eyre!("Ref pointed to something other than a commit"))?;

        let mut parent = match commit.parents().first() {
            Some(x) => x,
            None => return Ok(None),
        };

        distance -= 1;

        while distance > 0 {
            commit = repo
                .database
                .load(parent)?
                .into_commit()
                .ok_or_else(|| eyre!("Ref pointed to something other than a commit"))?;

            parent = match commit.parents().first() {
                Some(x) => x,
                None => return Ok(None),
            };

            distance -= 1;
        }

        assert_eq!(distance, 0);

        Refname::Sha1(parent.clone()).resolve(repo)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Refname {
    BranchTag(String),
    Sha1(Digest),
    PartialSha1(String),
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

        if input.chars().all(|c| c.is_ascii_hexdigit()) && input.len() <= 40 {
            return Ok(Self::PartialSha1(input.to_owned()));
        }

        if !is_valid_ref_name(input) {
            return Err(eyre!("Invalid ref name: {}", input));
        }

        Ok(Self::BranchTag(input.to_owned()))
    }

    pub fn resolve(&self, repo: &Repo) -> Result<Option<Digest>> {
        fn branchtag(name: &str, repo: &Repo) -> Result<Option<Digest>> {
            let oid = match repo.read_ref(name)? {
                Some(x) => x,
                None => return Ok(None),
            };

            Ok(Some(oid))
        }

        match self {
            Refname::Head => {
                let oid = match repo.read_head()? {
                    Some(x) => x,
                    None => return Ok(None),
                };

                Ok(Some(oid))
            }

            Refname::Sha1(oid) => {
                if repo.database.contains(oid) {
                    match repo.database.load(oid)? {
                        LoadedItem::Commit(_) => Ok(Some(oid.clone())),
                        other => Err(eyre!(
                            "Ref '{:x}' pointed to something other than a commit: {}",
                            oid,
                            other.kind()
                        )),
                    }
                } else {
                    Ok(None)
                }
            }

            Refname::PartialSha1(candidate) => {
                let entries = repo.database.entries();

                let candidates = entries
                    .into_iter()
                    .filter_map(|entry| {
                        if entry.to_hex().starts_with(candidate) {
                            Some(entry)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                match candidates.as_slice() {
                    // No candidates - treat as a branch / tag name
                    [] => branchtag(candidate, repo),

                    // One candidate - Found it!
                    [oid] => match repo.database.load(oid)? {
                        LoadedItem::Commit(_) => Ok(Some(oid.clone())),
                        other => Err(eyre!(
                            "Ref '{:x}' pointed to something other than a commit: {}",
                            oid,
                            other.kind()
                        )),
                    },

                    // Multiple candidates - Ambiguous, tell user to be more specific
                    _ => {
                        println!(
                            "Ambiguous ref name: `{}` matches multiple candidates:",
                            candidate
                        );
                        for candidate in candidates {
                            let kind = repo.database.load(&candidate)?.kind();
                            println!("  {:x} {}", candidate, kind);
                        }
                        Err(eyre!("Ambiguous ref name"))
                    }
                }
            }

            Refname::BranchTag(name) => branchtag(name, repo),
        }
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn head() {
        let rev = "HEAD";
        let rev = Rev::parse(rev).unwrap();
        assert_eq!(rev.refname, Refname::Head);
    }

    #[test]
    fn sha1() {
        let rev = "ffc1c862714edb677d6f467902cf2e406eee22ce";
        let rev = Rev::parse(rev).unwrap();
        let dig = Digest::from_str("ffc1c862714edb677d6f467902cf2e406eee22ce").unwrap();
        assert_eq!(rev.refname, Refname::Sha1(dig));
    }

    #[test]
    fn branch_tag() {
        let branch_tag = ["master", "main", "origin/main", "v1.0.0"];
        for name in branch_tag {
            dbg!(name);
            let rev = Rev::parse(name).unwrap();
            assert_eq!(rev.refname, Refname::BranchTag(name.to_owned()));
        }
    }

    #[test]
    fn parents() {
        let parents = ["HEAD^", "HEAD^^"];
        let expected = [
            Rev {
                refname: Refname::Head,
                distance: 1,
            },
            Rev {
                refname: Refname::Head,
                distance: 2,
            },
        ];

        for (rev, expected) in parents.into_iter().zip(expected) {
            dbg!(&rev);
            let rev = Rev::parse(rev).unwrap();
            assert_eq!(rev, expected);
        }
    }

    #[test]
    fn ancestors() {
        let ancestors = ["HEAD~1", "HEAD~2", "HEAD~3", "HEAD~1012123119"];
        let expected = [
            Rev {
                refname: Refname::Head,
                distance: 1,
            },
            Rev {
                refname: Refname::Head,
                distance: 2,
            },
            Rev {
                refname: Refname::Head,
                distance: 3,
            },
            Rev {
                refname: Refname::Head,
                distance: 1012123119,
            },
        ];
        for (rev, expected) in ancestors.into_iter().zip(expected) {
            dbg!(&rev);
            let rev = Rev::parse(rev).unwrap();
            assert_eq!(rev, expected);
        }
    }

    #[test]
    fn complex() {
        let complex = "HEAD~12^^~2";
        let expected = Rev {
            refname: Refname::Head,
            distance: 16,
        };
        let rev = Rev::parse(complex).unwrap();
        assert_eq!(rev, expected,);
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
        let expected = [
            Rev {
                refname: Refname::Head,
                distance: 1,
            },
            Rev {
                refname: Refname::Head,
                distance: 42,
            },
            Rev {
                refname: Refname::BranchTag("master".to_owned()),
                distance: 2,
            },
            Rev {
                refname: Refname::PartialSha1("abc123".to_owned()),
                distance: 3,
            },
        ];

        for (rev, expected) in book_testcases.into_iter().zip(expected) {
            dbg!(&rev);
            let rev = Rev::parse(rev).unwrap();
            assert_eq!(rev, expected);
        }
    }
}

#[cfg(test)]
mod evaluator_tests {
    use camino::Utf8Path;
    use tempdir::TempDir;

    use crate::test::{COMMIT_EMAIL, COMMIT_NAME};

    use super::*;

    fn init_repo(dir: &Utf8Path) -> Result<Repo> {
        std::env::set_var("RIT_AUTHOR_NAME", COMMIT_NAME);
        std::env::set_var("RIT_AUTHOR_EMAIL", COMMIT_EMAIL);

        Repo::init_default(dir)?;

        crate::create_test_files!(dir, ["file0"]);

        let mut repo = Repo::open(dir.to_owned())?;
        repo.add_all()?;
        repo.commit("zero")?;
        crate::create_test_files!(dir, ["file1"]);
        repo.add_all()?;
        repo.commit("one")?;
        crate::create_test_files!(dir, ["file2"]);
        repo.add_all()?;
        repo.commit("two")?;
        crate::create_test_files!(dir, ["file3"]);
        repo.add_all()?;
        repo.commit("three")?;
        crate::create_test_files!(dir, ["file4"]);
        repo.add_all()?;
        let commit_id = dbg!(repo.commit("four")?);

        repo.create_branch("master", &commit_id)?;

        Ok(repo)
    }

    #[test]
    fn works() -> Result<()> {
        let dir = TempDir::new("")?;
        let dir = dir.path();
        let dir = Utf8Path::from_path(dir).unwrap();

        let repo = init_repo(dir)?;

        let rev = Rev::parse("HEAD")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "four");

        let tid = commit.tree_id().clone();
        let tid = tid.to_hex();
        let rev = Rev::parse(&tid)?;
        assert!(rev.resolve(&repo).is_err());

        let three_oid = commit.parents().first().unwrap();
        let three_oid = three_oid.to_hex();
        let rev = Rev::parse(&three_oid)?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "three");

        let three_oid_frag = &three_oid[..6];
        let rev = Rev::parse(three_oid_frag)?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "three");

        let rev = Rev::parse("HEAD^")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "three");

        let rev = Rev::parse("HEAD^^")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "two");

        let rev = Rev::parse("HEAD~0")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "four");

        let rev = Rev::parse("HEAD~1")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "three");

        let rev = Rev::parse("HEAD~2")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "two");

        let rev = Rev::parse("HEAD~3")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "one");

        let rev = Rev::parse("HEAD~4")?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "zero");

        let rev = Rev::parse("HEAD~5")?;
        assert_eq!(rev.resolve(&repo)?, None);

        let null = Digest::NULL;
        let rev = Rev::parse(&null.to_hex())?;
        assert_eq!(rev.resolve(&repo)?, None);

        let rev = Rev::parse("master")?;
        let oid = rev.resolve(&repo)?.unwrap();
        dbg!(&oid);
        let commit = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), "four");

        let shortened = oid.to_hex()[..].to_owned();

        // Check that the shortened oid resolves to the same commit.
        let rev = Rev::parse(&shortened)?;
        let oid = rev.resolve(&repo)?.unwrap();
        let commit_from_short = repo.database.load(&oid)?.into_commit().unwrap();
        assert_eq!(commit.message(), commit_from_short.message());

        let rev = Rev::parse("main")?;
        assert_eq!(rev.resolve(&repo)?, None);

        Ok(())
    }
}
