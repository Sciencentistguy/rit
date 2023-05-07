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
            let path = self.git_dir.join(contents);
            if !path.exists() {
                if !self.database.any(|item| item.is_commit())? {
                    // an empty repo can have a dangling HEAD and that is fine
                    return Ok(None);
                }
                Err(eyre!("HEAD points to non-existstant ref: {}", path))
            } else {
                let contents = std::fs::read_to_string(path)?;
                let contents = contents.trim();
                Ok(Some(Digest::from_str(contents)?))
            }
        } else {
            let digest = Digest::from_str(contents)
                .wrap_err(eyre!("Unexpected HEAD contents: {contents}"))?;
            Ok(Some(digest))
        }
    }
}
