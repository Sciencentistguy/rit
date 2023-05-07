use color_eyre::eyre::eyre;

use crate::Result;

use super::*;

mod nom {
    use std::str::FromStr;

    use bstr::ByteSlice;
    use nom::{
        bytes::complete::{tag, take, take_till, take_until, take_while},
        Parser,
    };
    use nom_supreme::ParserExt;

    use crate::{
        commit::{Commit, GpgSig, Signature, Timestamp},
        digest::Digest,
    };

    pub type Input<'a> = &'a [u8];
    pub type Result<'a, O> = nom::IResult<Input<'a>, O, nom::error::VerboseError<Input<'a>>>;
    pub type BitResult<'a, O> = nom::IResult<Input<'a>, O>;

    pub(super) fn parse_commit(i: Input) -> Result<Commit> {
        let (i, tree) = parse_tree.context("Tree").parse(i)?;
        let (i, parents) = parse_parents.context("Parents").parse(i)?;
        let (i, author) = parse_author.context("Author").parse(i)?;
        let (i, committer) = parse_committer.context("Committer").parse(i)?;
        let (i, gpgsig) = parse_gpgsig.context("GpgSig").parse(i)?;
        let (i, message) = parse_message.context("Message").parse(i)?;

        Ok((
            i,
            Commit {
                tree_id: tree,
                parents,
                author,
                committer,
                gpgsig,
                message: message.to_str().unwrap().to_owned(),
            },
        ))
    }

    fn parse_message(i: Input) -> Result<&[u8]> {
        let (i, _) = take_while(|x: u8| x.is_ascii_whitespace())(i)?;
        Ok((b"", i.trim()))
    }

    fn parse_gpgsig(i: Input) -> Result<Option<GpgSig>> {
        let (i, header) = tag(b"gpgsig ").opt().parse(i)?;
        if header.is_none() {
            return Ok((i, None));
        }

        let (i, _) = take_until("END PGP SIGNATURE")(i)?;
        let (i, _) = take_till(|x| x == b'\n')(i)?;
        let (i, _) = tag(b"\n")(i)?;

        Ok((i, Some(GpgSig)))
    }

    fn parse_signature(i: Input) -> Result<Signature> {
        let (i, name) = take_till(|b| b == b'<').context("name").parse(i)?;
        let name = name.trim().to_str().unwrap();
        let (i, _) = tag(b"<")(i)?;
        let (i, email) = take_till(|b| b == b'>').context("email").parse(i)?;
        let email = email.trim().to_str().unwrap();
        let (i, _) = tag(b"> ")(i)?;

        // the rest of the string, up to \n, is the unix timestamp, and the offset "%s %z"
        let (i, time) = take_till(|b| b == b'\n')
            .context("timestamp/offset")
            .parse(i)?;
        let time = time.trim().to_str().unwrap();
        let time = Timestamp::from_git(time).unwrap();

        Ok((
            i,
            Signature {
                name: name.to_owned(),
                email: email.to_owned(),
                when: time,
            },
        ))
    }

    pub(super) fn parse_author(i: Input) -> Result<Signature> {
        let (i, _) = tag(b"author ")(i)?;
        let (i, ret) = parse_signature(i)?;
        let (i, _) = tag(b"\n")(i)?;
        Ok((i, ret))
    }

    fn parse_committer(i: Input) -> Result<Signature> {
        let (i, _) = tag(b"committer ")(i)?;
        let (i, ret) = parse_signature(i)?;
        let (i, _) = tag(b"\n")(i)?;
        Ok((i, ret))
    }

    fn parse_tree(i: Input) -> Result<Digest> {
        let (i, _) = tag(b"tree ")(i)?;
        let (i, tree) = take(40usize)(i)?;
        let tree = Digest::from_str(tree.to_str().unwrap()).unwrap();
        let (i, _) = tag(b"\n")(i)?;
        Ok((i, tree))
    }

    fn parse_parent(i: Input) -> Result<Digest> {
        let (i, _) = tag(b"parent ")(i)?;
        let (i, parent) = take(40usize)(i)?;
        let parent = Digest::from_str(parent.to_str().unwrap()).unwrap();
        let (i, _) = tag(b"\n")(i)?;
        Ok((i, parent))
    }

    fn parse_parents(mut i: Input) -> Result<Vec<Digest>> {
        let mut parents = Vec::new();

        while let (new_i, Some(parent)) = nom::combinator::opt(parse_parent)(i)? {
            i = new_i;
            parents.push(parent);
        }

        Ok((i, parents))
    }
}

impl Commit {
    /// Parse a decompressed commit.
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
        match nom::parse_commit(bytes) {
            Ok((_, commit)) => Ok(commit),
            Err(e) => Err(eyre!("Failed to parse commit: {e:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::{DateTime, FixedOffset, NaiveDate};

    use super::*;

    #[test]
    fn test_parse_signature() {
        let input = "author Jamie Quigley <jamie@quigley.xyz> 1658312219 +0100\n";
        let (_, signature) = nom::parse_author(input.as_bytes()).unwrap();

        assert_eq!(signature.name, "Jamie Quigley");
        assert_eq!(signature.email, "jamie@quigley.xyz");

        let expected_ts = {
            let ndt = NaiveDate::from_ymd_opt(2022, 7, 20)
                .unwrap()
                .and_hms_opt(11, 16, 59)
                .unwrap();
            let offset = FixedOffset::east(60 * 60 /*1 hour*/);
            DateTime::<FixedOffset>::from_local(ndt, offset)
        };

        assert_eq!(signature.when.0, expected_ts,);
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
