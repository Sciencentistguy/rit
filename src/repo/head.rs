use std::str::FromStr;

use crate::digest::Digest;
use crate::Result;

use color_eyre::eyre::{eyre, Context};

impl super::Repo {
    pub fn read_head(&self) -> Result<Option<Digest>> {
        if !self.head_path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&self.head_path)?;
        let contents = contents.trim();
        if let Some(contents) = contents.strip_prefix("ref: ") {
            let path = self.dir.join(".git").join(contents);
            dbg!(&path);
            if !path.exists() {
                Err(eyre!("HEAD points to non-existstant ref: {}", path))
            } else {
                let contents = std::fs::read_to_string(path)?;
                let contents = contents.trim();
                Ok(Some(Digest::from_str(contents)?))
            }
        } else {
            let digest = Digest::from_str(contents)
                .wrap_err(format!("Unexpected HEAD contents: {contents}"))?;
            Ok(Some(digest))
        }
    }
}
