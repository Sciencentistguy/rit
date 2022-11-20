mod parse;
mod write;

use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashSet;

use tracing::trace;

use crate::digest::Digest;
use crate::filemode::FileMode;
use crate::util::Descends;
use crate::Result;

struct IndexHeader {
    magic: [u8; 4],
    version: u32,
    num_entries: u32,
}

impl IndexHeader {
    fn has_valid_magic(&self) -> bool {
        &self.magic == b"DIRC"
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IndexEntry {
    ctime_s: u32,
    ctime_n: u32,

    mtime_s: u32,
    mtime_n: u32,

    dev: u32,
    ino: u32,

    mode: FileMode,

    uid: u32,
    gid: u32,
    siz: u32,
    oid: Digest,
    flags: u16,
    name: String,
}

impl IndexEntry {
    pub fn digest(&self) -> &Digest {
        &self.oid
    }

    pub fn mode(&self) -> FileMode {
        self.mode
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &Utf8Path {
        Utf8Path::new(&self.name)
    }

    pub fn parents(&self) -> Vec<&Utf8Path> {
        let mut v = self.path().descends();
        v.pop();
        v
    }
}

impl Ord for IndexEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}
impl PartialOrd for IndexEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl IndexEntry {
    const MAX_PATH_SIZE: u16 = 0xfff;

    pub fn new(path: &Utf8Path, oid: &Digest, stat: libc::stat) -> Result<Self> {
        let name = path.as_str().to_owned();

        let flags = path
            .as_os_str()
            .len()
            .try_into()
            .unwrap_or(Self::MAX_PATH_SIZE);

        let mode = FileMode::from(&stat);

        Ok(Self {
            ctime_s: stat.st_ctime.try_into()?,
            ctime_n: stat.st_ctime_nsec.try_into()?,
            mtime_s: stat.st_mtime.try_into()?,
            mtime_n: stat.st_mtime_nsec.try_into()?,
            dev: stat.st_dev.try_into()?,
            ino: stat.st_ino.try_into()?,
            mode,
            uid: stat.st_uid,
            gid: stat.st_gid,
            siz: stat.st_size.try_into()?,
            oid: oid.clone(),
            flags,
            name,
        })
    }

    pub fn oid(&self) -> &Digest {
        &self.oid
    }

    pub fn stat_matches(&self, stat: &libc::stat) -> bool {
        // Fine to cast unconditionally, u32 will always fit in i64
        (self.siz == 0 || self.siz as i64 == stat.st_size) && (FileMode::from(stat) == self.mode)
    }

    pub fn times_match(&self, stat: &libc::stat) -> bool {
        self.ctime_s as i64 == stat.st_ctime
            && self.ctime_n as i64 == stat.st_ctime_nsec
            && self.mtime_s as i64 == stat.st_mtime
            && self.mtime_n as i64 == stat.st_mtime_nsec
    }
}

struct Index {
    header: IndexHeader,
    entries: Vec<IndexEntry>,
    // oid: Digest,
}

impl Index {
    fn from_entries(entries: &[IndexEntry]) -> Self {
        let header = IndexHeader {
            magic: *b"DIRC",
            version: 2,
            num_entries: entries
                .len()
                .try_into()
                .expect("The number of entries should fit in a u32"),
        };

        Self {
            header,
            entries: entries.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct IndexWrapper {
    path: Utf8PathBuf,
    //FIXME: this could be a Cow
    entries: Vec<IndexEntry>,
}

impl IndexWrapper {
    pub fn open(git_folder: &Utf8Path) -> Self {
        let index_path = git_folder.join("index");
        let entries = (|| -> Result<Vec<IndexEntry>> {
            let current_index = std::fs::read(&index_path)?;
            let current_index = parse::parse_index(&current_index)?;
            Ok(current_index.entries)
        })()
        .unwrap_or_else(|_| Vec::new());

        trace!(?index_path, "Opened index with {} entries", entries.len());

        Self {
            path: index_path,
            entries,
        }
    }

    pub fn add(&mut self, path: &Utf8Path, oid: &Digest, stat: libc::stat) {
        trace!(?path, "Adding entry to index");
        let entry = IndexEntry::new(path, oid, stat).unwrap();

        self.discard_conflicts(&entry);
        self.entries.push(entry);

        self.entries.sort_unstable();
    }

    pub fn write_out(&self) -> Result<()> {
        let index = Index::from_entries(&self.entries);
        let index = write::write_index(&index);
        std::fs::write(&self.path, index)?;
        Ok(())
    }

    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// Discard all entries that conflict with the given entry.
    /// e.g. if a file is added at `foo/bar` then `/foo/bar/baz` will be removed.
    ///
    /// Note: Does not preserve order
    fn discard_conflicts(&mut self, entry: &IndexEntry) {
        let mut to_remove = HashSet::new();

        for dir in entry.parents() {
            let idx = self.entries.iter().position(|e| e.path() == dir);
            if let Some(idx) = idx {
                to_remove.insert(idx);
            }
        }

        'outer: for (i, old_entry) in self.entries().iter().enumerate() {
            for path in old_entry.parents() {
                if path == entry.path() {
                    to_remove.insert(i);
                    continue 'outer;
                }
            }
        }

        for idx in to_remove {
            trace!(entry = ?self.entries[idx].path(), "Removing entry due to conflict");
            self.entries.swap_remove(idx);
        }
    }

    pub fn get_entry_by_path(&self, path: &Utf8Path) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.path() == path)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::Permissions;
    use std::os::unix::prelude::PermissionsExt;
    use std::process::{Command, Stdio};

    use camino::Utf8Path;
    use tempdir::TempDir;

    use super::{parse::*, write::*};
    use crate::repo::Repo;
    use crate::Result;

    #[test]
    #[ignore = "Doesn't work in CI"]
    /// Parse an index from the filesystem and write it back out. The generated file should be
    /// identical (modulo extensions, and therefore also the oid).
    ///
    /// Note that this is disabled by default, as it relys on an existing, large repository (the
    /// provided path is a nixpkgs checkout).
    fn read_write_index() {
        const TEST_GIT_REPO_PATH: &str = "/home/jamie/Git/nixpkgs-official/.git/index";
        let bytes = std::fs::read(TEST_GIT_REPO_PATH).unwrap();
        let idx = parse_index(&bytes).unwrap();

        for e in &idx.entries {
            println!("{:?}", e.name);
        }

        let new_bytes = write_index(&idx);
        if new_bytes.len() < bytes.len() {
            println!("Dropped extensions, oid will not match...");
            let len = new_bytes.len() - 20;
            assert_eq!(bytes[..len], new_bytes[..len]);
        } else {
            assert_eq!(bytes, new_bytes);
        }
    }

    #[test]
    /// Place files in the directory. Using rit, init the directory as a repo, and add the files.
    /// Read the genreated `.git/index` into a Vec.
    ///
    /// Then, delete `.git`, and perform the same operations (on the same files) using git.
    /// Read the generated `.git/index` into a Vec.
    ///
    /// These should be indentical
    fn generate_and_read_index() -> Result<()> {
        let dir = TempDir::new("")?;
        let dir = dir.path();
        let dir = Utf8Path::from_path(dir).unwrap();

        // Test files:
        // - file1: a normal file, chmod 644 (should be stored as REGULAR)
        // - file2: a normal file, chmod 755 (should be stored as EXECUTABLE)
        // - file3: a normal file, chmod 655 (should be stored as REGULAR (644))
        // - a/b/c.txt: a file in a directory
        crate::create_test_files!(dir, ["file1", "file2", "file3", "a/b/c.txt"]);
        std::fs::set_permissions(dir.join("file2"), Permissions::from_mode(0o100755))?;
        std::fs::set_permissions(dir.join("file3"), Permissions::from_mode(0o100655))?;

        Repo::init_default(dir)?;
        let mut rit_repo = Repo::open(dir.to_owned())?;
        rit_repo.add_all()?;
        let rit_index = std::fs::read(dir.join(".git/index"))?;

        std::fs::remove_dir_all(dir.join(".git"))?;

        Command::new("git")
            .arg("init")
            .current_dir(&dir)
            .stdout(Stdio::null())
            .status()
            .unwrap();

        Command::new("git")
            .arg("add")
            .arg("--all")
            .current_dir(&dir)
            .stdout(Stdio::null())
            .status()?;

        let git_index = std::fs::read(dir.join(".git/index"))?;

        assert_eq!(rit_index, git_index);

        Ok(())
    }
}
