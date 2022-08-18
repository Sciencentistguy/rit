use camino::{Utf8Path, Utf8PathBuf};
use tap::Tap;

use crate::{blob::Blob, digest::Digest, filemode::FileMode, storable::Storable, Result};

use super::{
    status::{Change, Status},
    Repo,
};

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
            match change {
                Change::Modified | Change::Removed => self.diff_file(path)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn diff_file(&self, path: &Utf8Path) -> Result<()> {
        let (a, b) = DiffTarget::new(path, self)?;

        println!("diff --git {} {}", a.path(), b.path());

        self.print_diff_mode(&a, &b);

        self.print_diff_content(&a, &b);

        Ok(())
    }

    fn print_diff_mode(&self, a: &DiffTarget, b: &DiffTarget) {
        if b.is_removed() {
            println!("deleted file mode {:o}", a.mode().unwrap());
        } else if a.mode() != b.mode() {
            println!("old mode {:o}", a.mode().unwrap());
            println!("new mode {:o}", b.mode().unwrap());
        }
    }

    fn print_diff_content(&self, a: &DiffTarget, b: &DiffTarget) {
        if b.is_removed() {
            println!("index {}..{}", a.oid().short(), b.oid().short(),);
        } else {
            println!(
                "index {}..{} {:o}",
                a.oid().short(),
                b.oid().short(),
                a.mode().unwrap()
            );
        }
        println!("--- {}", a.path());
        println!("+++ {}", b.path());
    }
}

enum DiffTarget {
    Removed,
    Modified {
        oid: Digest,
        mode: FileMode,
        path: Utf8PathBuf,
    },
}

pub const NULL_PATH: &str = "/dev/null";

impl DiffTarget {
    /// Construct a pair of DiffTargets, `a`, and `b`. The first is guaranteed to be of variant
    /// [`Modified`], whereas the latter may be [`Removed`].
    ///
    /// [`Removed`]: DiffTarget::Removed
    /// [`Modified`]: DiffTarget::Modified
    fn new(path: &Utf8Path, repo: &Repo) -> Result<(Self, Self)> {
        let entry = repo
            .index
            .get_entry_by_path(path)
            .expect("file is not in index");
        let a_oid = entry.oid().clone();
        let a_mode = entry.mode();
        let a_path = Utf8Path::new("a").join(path);

        let a = Self::Modified {
            oid: a_oid,
            mode: a_mode,
            path: a_path,
        };

        if !path.exists() {
            Ok((a, Self::Removed))
        } else {
            let file = std::fs::read(path)?;
            let blob = Blob::new(file);
            let b_oid = blob.oid(&blob.format());
            let b_path = Utf8Path::new("b").join(path);
            let b_mode = FileMode::from(&Repo::stat_file(path)?.unwrap());
            let b = Self::Modified {
                oid: b_oid,
                mode: b_mode,
                path: b_path,
            };
            Ok((a, b))
        }
    }

    fn oid(&self) -> &Digest {
        match self {
            DiffTarget::Removed => &Digest::NULL_DIGEST,
            DiffTarget::Modified { oid, .. } => oid,
        }
    }

    fn path(&self) -> &Utf8Path {
        match self {
            DiffTarget::Removed => Utf8Path::new(NULL_PATH),
            DiffTarget::Modified { path, .. } => path,
        }
    }

    fn mode(&self) -> Option<FileMode> {
        match self {
            DiffTarget::Removed => None,
            DiffTarget::Modified { mode, .. } => Some(*mode),
        }
    }

    /// Returns `true` if the diff target is [`Removed`].
    ///
    /// [`Removed`]: DiffTarget::Removed
    #[must_use]
    fn is_removed(&self) -> bool {
        matches!(self, Self::Removed)
    }
}
