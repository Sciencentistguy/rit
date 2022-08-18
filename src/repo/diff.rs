use camino::Utf8Path;
use tap::Tap;

use crate::{blob::Blob, storable::Storable, Result};

use super::status::{Change, Status};

impl super::Repo {
    pub fn diff(&self) -> Result<()> {
        let status = match Status::new(self)? {
            Some(x) => x,
            None => return Ok(()),
        };

        let changes = status
            .get_statuses()?
            .tap_mut(|v| v.sort_unstable_by_key(|x| x.0));

        for (path, change) in changes {
            if change == Change::Modified {
                self.diff_file(path)?;
            }
        }

        Ok(())
    }

    fn diff_file(&self, path: &Utf8Path) -> Result<()> {
        let entry = self
            .index
            .get_entry_by_path(path)
            .expect("file is not in index");
        let a_oid = entry.oid();
        let a_mode = entry.mode();
        let a_path = Utf8Path::new("a").join(path);

        let file = std::fs::read(path)?;
        let blob = Blob::new(file);
        let b_oid = blob.oid(&blob.format());
        let b_path = Utf8Path::new("b").join(path);

        /*
        puts "diff --git #{ a_path } #{ b_path }"
        puts "index #{ short a_oid }..#{ short b_oid } #{ a_mode }"
        puts "--- #{ a_path }"
        puts "+++ #{ b_path }
                 */

        println!("diff --git {} {}", a_path, b_path);
        println!("index {}..{} {:o}", a_oid.short(), b_oid.short(), a_mode);
        println!("--- {}", a_path);
        println!("+++ {}", b_path);

        Ok(())
    }
}
