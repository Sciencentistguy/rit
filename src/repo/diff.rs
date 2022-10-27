use camino::{Utf8Path, Utf8PathBuf};
use tap::Tap;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
    blob::Blob, diff::EditKind, digest::Digest, filemode::FileMode, index::IndexEntry,
    storable::Storable, tree::Tree, Result,
};

use super::{
    status::{Change, Status},
    Repo,
};

pub enum DiffMode {
    WorktreeIndex,
    IndexHead,
}

impl super::Repo {
    pub fn diff(&self, mode: DiffMode) -> Result<()> {
        let status = match Status::new(self)? {
            Some(x) => x,
            None => return Ok(()),
        };

        let changes = status
            .get_statuses()?
            .tap_mut(|v| v.sort_unstable_by_key(|x| x.0));

        match mode {
            DiffMode::WorktreeIndex => {
                for (path, change) in changes {
                    match change {
                        Change::Modified | Change::Removed => {
                            let a = DiffTarget::from_file(path)?;
                            let b = DiffTarget::from_index(path, self)?;
                            self.diff_files(a, b)?
                        }
                        _ => {}
                    }
                }
            }
            DiffMode::IndexHead => {
                let tree = status.tree();

                for (path, change) in changes {
                    match change {
                        Change::IndexModified | Change::IndexAdded | Change::IndexRemoved => {
                            let a = DiffTarget::from_index(path, self)?;
                            let b = DiffTarget::from_head(path, self, tree)?;
                            self.diff_files(a, b)?
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn diff_files(&self, a: DiffTarget, b: DiffTarget) -> Result<()> {
        println!("diff --git {} {}", a.path(), b.path());

        self.print_diff_mode(&a, &b);

        self.print_diff_content(&a, &b);

        Ok(())
    }

    fn print_diff_mode(&self, a: &DiffTarget, b: &DiffTarget) {
        if a.is_removed() {
            println!("added file mode {:o}", b.mode().unwrap());
        } else if b.is_removed() {
            println!("deleted file mode {:o}", a.mode().unwrap());
        } else if a.mode() != b.mode() {
            println!("old mode {:o}", a.mode().unwrap());
            println!("new mode {:o}", b.mode().unwrap());
        }
    }

    fn print_diff_content(&self, a: &DiffTarget, b: &DiffTarget) {
        if a.mode() != b.mode() {
            println!("index {}..{}", a.oid().short(), b.oid().short());
        } else {
            assert!(a.mode().is_some());
            println!(
                "index {}..{} {:o}",
                a.oid().short(),
                b.oid().short(),
                a.mode().unwrap()
            );
        }
        println!("--- {}", a.path());
        println!("+++ {}", b.path());

        let a = std::str::from_utf8(a.data());
        let b = std::str::from_utf8(b.data());

        if a.is_err() || b.is_err() {
            unimplemented!("invalid utf-8, assuming binary file, which is NYI")
        }

        let a = a.unwrap().lines().collect::<Vec<_>>();
        let b = b.unwrap().lines().collect::<Vec<_>>();

        let edits = crate::diff::diff(&a, &b);

        let hunks = crate::diff::hunks(&edits);

        let mut writer = StandardStream::stdout(ColorChoice::Auto);

        for hunk in hunks {
            writer
                .set_color(ColorSpec::new().set_fg(Some(Color::Ansi256(244))))
                .unwrap();
            println!("{}", hunk.header());
            writer.reset().unwrap();
            for edit in hunk.edits() {
                match edit.kind() {
                    EditKind::Insert => writer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                        .unwrap(),
                    EditKind::Delete => writer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Red)))
                        .unwrap(),
                    EditKind::Equal => writer.reset().unwrap(),
                };

                println!("{}", edit);
            }
        }
    }
}

enum DiffTarget {
    Removed,
    Modified {
        oid: Digest,
        mode: FileMode,
        path: Utf8PathBuf,
        data: Vec<u8>,
    },
}

pub const NULL_PATH: &str = "/dev/null";

impl DiffTarget {
    fn from_file(path: &Utf8Path) -> Result<Self> {
        if !path.exists() {
            Ok(Self::Removed)
        } else {
            let bytes = std::fs::read(path)?;
            let blob = Blob::new(bytes);
            let formatted = blob.format();

            let oid = blob.oid(&formatted);
            let mode = FileMode::from(&Repo::stat_file(path)?.unwrap());
            let path = Utf8Path::new("b").join(path);
            Ok(Self::Modified {
                oid,
                mode,
                path,
                data: blob.into_data(),
            })
        }
    }

    fn from_index(path: &Utf8Path, repo: &Repo) -> Result<Self> {
        let entry = match repo.index.get_entry_by_path(path) {
            Some(x) => x,
            None => return Ok(Self::Removed),
        };
        Self::from_entry(path, repo, entry)
    }

    fn from_head(path: &Utf8Path, repo: &Repo, tree: &Tree) -> Result<Self> {
        let entry = match tree.get_entry(path.as_str()) {
            Some(x) => x,
            None => return Ok(Self::Removed),
        };
        Self::from_entry(path, repo, entry)
    }

    fn from_entry(path: &Utf8Path, repo: &Repo, entry: &IndexEntry) -> Result<Self> {
        let oid = entry.oid().clone();
        let mode = entry.mode();
        let path = Utf8Path::new("a").join(path);
        let blob = repo
            .database
            .load(&oid)?
            .into_blob()
            .expect("file oid should be blob");

        Ok(Self::Modified {
            oid,
            mode,
            path,
            data: blob.into_data(),
        })
    }

    fn oid(&self) -> &Digest {
        match self {
            DiffTarget::Removed => &Digest::NULL,
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

    fn data(&self) -> &[u8] {
        match self {
            DiffTarget::Removed => &[],
            DiffTarget::Modified { data, .. } => data,
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
