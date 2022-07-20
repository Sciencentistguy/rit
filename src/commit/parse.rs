
use std::str::FromStr;

use color_eyre::eyre::eyre;
use memchr::memmem;
use once_cell::sync::Lazy;
use regex::bytes::Regex;

use crate::digest::Digest;
use crate::Result;

use super::*;

impl Commit {
    /// Parse a decompressec commit.
    ///
    /// ## Example
    /// ```text
    /// tree 090c4c5dd61d2e84c832c4cd306b66bf2fabc1f5
    /// parent e6a49274aa0893ce2e2928589100387aee220c5b
    /// parent 14a9d8464caef987f3b5c3cf26f56db825459abd
    /// author Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100
    /// committer Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100
    /// gpgsig -----BEGIN PGP SIGNATURE-----
    ///
    ///  iQEzBAABCAAdFiEEMLv/P6sLuz4ENfg8jo/2biro2XAFAmLX1h0ACgkQjo/2biro
    ///  2XC8yQf/eVwDZC0hZxMuPcHOsiDLa+f65tNvMA4k8edoQRp90+Z/o+ENewFnnKD5
    ///  64p0Rk6V7KTt9SGE5VJUnYzsNW0RU8js3fkHt+sE2qk4w7DcMLlROb/OLGYknRAq
    ///  Yu8cpBR00FU1atW7N3VHsJfVlfRKSwGORqdQae0JbEDipGK6y65aawWZO039W3+B
    ///  DLyb7RcVdvM3dshBfMcuVKBykh47pQdy02ShLGcbGbd3Akhhf27X1DySg+TLlC6m
    ///  pev5XIh9qRGEtEdtC/wCButaWl3vAnkzwUduZPD85eKMbwfejCJqCvlDK3v9q9wc
    ///  OjQIqDMzEhTbQSGLPlW/lb0jbxqkjg==
    ///  =JO5C
    ///  -----END PGP SIGNATURE-----
    ///
    /// Merge remote-tracking branch 'origin/renovate/clap-3.x' into develop
    ///
    /// ```
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        const LINES_FAIL_MSG: &str = "Unexpectedly reach end of commit. Is it valid?";

        let mut lines = bytes.split(|b| *b == b'\n');
        static TREEID_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"tree [[:xdigit:]]+$").unwrap());
        static PARENT_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"parent [[:xdigit:]]+$").unwrap());

        let tree_id: Digest = {
            let line = lines.next().expect(LINES_FAIL_MSG);
            if TREEID_REGEX.is_match(line) {
                let tree_id = std::str::from_utf8(&line[5..])?;
                Digest::from_str(tree_id)?
            } else {
                panic!("tree id not found");
            }
        };

        let mut parents = Vec::new();
        let author = {
            let line = loop {
                let line = lines.next().expect(LINES_FAIL_MSG);
                if PARENT_REGEX.is_match(line) {
                    let parent_id = std::str::from_utf8(&line[7..])?;
                    let parent_id = Digest::from_str(parent_id)?;
                    parents.push(parent_id);
                } else {
                    break line;
                }
            };
            Signature::parse(line)?
        };
        let commiter = Signature::parse(lines.next().expect(LINES_FAIL_MSG))?;
        let gpgsig = lines.next().and_then(|line| {
            if line.starts_with(b"gpgsig") {
                // XXX: gpgsig parsing NYI
                Some(GpgSig)
            } else {
                None
            }
        });

        if gpgsig.is_some() {
            loop {
                let line = lines.next().expect(LINES_FAIL_MSG);
                if memmem::find(line, b"END PGP SIGNATURE").is_some() {
                    break;
                }
            }
        }

        let message = lines.flatten().copied().collect::<Vec<_>>();
        let message = String::from_utf8(message)?;

        Ok(Self {
            tree_id,
            parents,
            author,
            committer: commiter,
            gpgsig,
            message,
        })
    }
}

impl Signature {
    /// Parse a signature line from a commit.
    ///
    /// ## Example
    /// `author Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100`
    /// `committer Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100`
    fn parse(bytes: &[u8]) -> Result<Self> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"(?:author|committer) ([\w\s]+) <(\S+)> (\d+) ((?:\+|-)\d+)").unwrap()
        });

        let groups = REGEX.captures(bytes).ok_or_else(|| {
            eyre!(
                "Invalid signature line: {}",
                std::string::String::from_utf8_lossy(bytes)
            )
        })?;

        let name = std::str::from_utf8(&groups[1])?.to_owned();
        let email = std::str::from_utf8(&groups[2])?.to_owned();
        let unix = std::str::from_utf8(&groups[3])?.parse()?;
        let offset = std::str::from_utf8(&groups[4])?.parse()?;

        Ok(Self {
            name,
            email,
            when: Timestamp { unix, offset },
        })
    }
}

impl GpgSig {
    /// Parse a gpgsig line from a commit.
    ///
    /// ## Example
    /// ```text
    /// gpgsig -----BEGIN PGP SIGNATURE-----
    ///
    ///  iQEzBAABCAAdFiEEMLv/P6sLuz4ENfg8jo/2biro2XAFAmLX1h0ACgkQjo/2biro
    ///  2XC8yQf/eVwDZC0hZxMuPcHOsiDLa+f65tNvMA4k8edoQRp90+Z/o+ENewFnnKD5
    ///  64p0Rk6V7KTt9SGE5VJUnYzsNW0RU8js3fkHt+sE2qk4w7DcMLlROb/OLGYknRAq
    ///  Yu8cpBR00FU1atW7N3VHsJfVlfRKSwGORqdQae0JbEDipGK6y65aawWZO039W3+B
    ///  DLyb7RcVdvM3dshBfMcuVKBykh47pQdy02ShLGcbGbd3Akhhf27X1DySg+TLlC6m
    ///  pev5XIh9qRGEtEdtC/wCButaWl3vAnkzwUduZPD85eKMbwfejCJqCvlDK3v9q9wc
    ///  OjQIqDMzEhTbQSGLPlW/lb0jbxqkjg==
    ///  =JO5C
    ///  -----END PGP SIGNATURE-----
    /// ```
    fn parse(_bytes: &[u8]) -> Result<Self> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_signature() {
        let input = "author Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100";

        let signature = Signature::parse(input.as_bytes()).unwrap();

        assert_eq!(signature.name, "Jamie Quigley");
        assert_eq!(signature.email, "jamie@quigley.xyz");

        assert_eq!(
            signature.when,
            Timestamp {
                unix: 1658312219,
                offset: 100
            }
        );
    }

    #[test]
    fn test_parse_commit() {
        let input = "tree 090c4c5dd61d2e84c832c4cd306b66bf2fabc1f5
parent e6a49274aa0893ce2e2928589100387aee220c5b
parent 14a9d8464caef987f3b5c3cf26f56db825459abd
author Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100
committer Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100
gpgsig -----BEGIN PGP SIGNATURE-----

 iQEzBAABCAAdFiEEMLv/P6sLuz4ENfg8jo/2biro2XAFAmLX1h0ACgkQjo/2biro
 2XC8yQf/eVwDZC0hZxMuPcHOsiDLa+f65tNvMA4k8edoQRp90+Z/o+ENewFnnKD5
 64p0Rk6V7KTt9SGE5VJUnYzsNW0RU8js3fkHt+sE2qk4w7DcMLlROb/OLGYknRAq
 Yu8cpBR00FU1atW7N3VHsJfVlfRKSwGORqdQae0JbEDipGK6y65aawWZO039W3+B
 DLyb7RcVdvM3dshBfMcuVKBykh47pQdy02ShLGcbGbd3Akhhf27X1DySg+TLlC6m
 pev5XIh9qRGEtEdtC/wCButaWl3vAnkzwUduZPD85eKMbwfejCJqCvlDK3v9q9wc
 OjQIqDMzEhTbQSGLPlW/lb0jbxqkjg==
 =JO5C
 -----END PGP SIGNATURE-----

Merge remote-tracking branch 'origin/renovate/clap-3.x' into develop

";

        let commit = Commit::parse(input.as_bytes()).unwrap();
        assert_eq!(
            commit.tree_id,
            Digest::from_str("090c4c5dd61d2e84c832c4cd306b66bf2fabc1f5").unwrap()
        );
        assert_eq!(
            commit.parents,
            vec![
                Digest::from_str("e6a49274aa0893ce2e2928589100387aee220c5b").unwrap(),
                Digest::from_str("14a9d8464caef987f3b5c3cf26f56db825459abd").unwrap(),
            ]
        );
        assert_eq!(commit.author.name, "Jamie Quigley");
        assert_eq!(commit.author.email, "jamie@quigley.xyz");
        assert_eq!(commit.committer.name, "Jamie Quigley");
        assert_eq!(commit.committer.email, "jamie@quigley.xyz");
        assert!(commit.gpgsig.is_some());
        assert_eq!(
            commit.message,
            "Merge remote-tracking branch 'origin/renovate/clap-3.x' into develop"
        );
    }
}
