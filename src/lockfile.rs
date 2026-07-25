use std::{
    borrow::{Borrow, BorrowMut}, fs::{File, OpenOptions}, io::ErrorKind, ops::{Deref, DerefMut}, path::{Path, PathBuf}, time::Duration
};

pub struct Lockfile<'a> {
    original_path: &'a Path,
    path: PathBuf,
    lockfile: Option<File>,
    file: Option<File>,
}

pub struct Guard<'a, 'b>
where
    'a: 'b,
{
    parent: &'b mut Lockfile<'a>,
}

impl DerefMut for Guard<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.parent.file.as_mut().unwrap()
    }
}

impl Deref for Guard<'_, '_> {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        self.parent.file.as_ref().unwrap()
    }
}

// impl BorrowMut<File> for Guard<'_, '_> {
    // fn borrow_mut(&mut self) -> &mut File {
        // self.parent.file.as_mut().unwrap()
    // }
// }

// impl Borrow<File> for Guard<'_, '_> {
    // fn borrow(&self) -> &File {
        // self.parent.file.as_ref().unwrap()
    // }
// }

impl Drop for Guard<'_, '_> {
    fn drop(&mut self) {
        self.parent.unlock();
    }
}

impl<'a> Lockfile<'a> {
    pub fn new<P: AsRef<Path>>(path: &'a P) -> Self {
        let original_path = path.as_ref();
        let mut path = original_path.to_owned();

        path.as_mut_os_string().push(".lock");
        Self {
            original_path,
            path,
            lockfile: None,
            file: None,
        }
    }

    pub fn try_lock(&mut self) -> Option<Guard<'a, '_>> {
        let mut lf = OpenOptions::new();
        lf.write(true);
        lf.create_new(true);
        let mut oo = OpenOptions::new();
        oo.write(true);
        oo.create(true);
        match lf.open(&self.path) {
            Ok(x) => {
                self.lockfile = Some(x);
                self.file = Some(oo.open(self.original_path).unwrap());
                Some(Guard { parent: self })
            }
            Err(e) => match e.kind() {
                ErrorKind::AlreadyExists => None,
                _ => panic!("whoops other error: {e:?}"),
            },
        }
    }

    pub fn lock<'b>(&'b mut self) -> Guard<'a, 'b> {
        let mut counter = 0;

        loop {
            counter += 1;
            if counter > 20 {
                panic!(
                    "Failed to lock path {}. Is another git/rit process running?",
                    self.original_path.display()
                )
            }

            // SAFETY: `mut_self` creates a fresh borrow per iteration.
            // If `try_lock()` fails (returns None), `mut_self` goes out of scope,
            // leaving `self` unborrowed for the next iteration.
            let mut_self = unsafe { &mut *(self as *mut Self) };
            if let Some(guard) = mut_self.try_lock() {
                return guard;
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn unlock(&mut self) {
        if self.lockfile.is_some() {
            std::fs::remove_file(&self.path).expect("remove_file should succeed");
        }
        self.file = None;
        self.lockfile = None;
    }
}

#[test]
fn test_lockfile() {
    use tempdir::TempDir;

    let dir = TempDir::new("").unwrap();
    let path = dir.path();
    let path = path.join("testfile");
    let mut lf = Lockfile::new(&path);
    let guard = lf.lock();
    let mut lf2 = Lockfile::new(&path);
    assert!(lf2.try_lock().is_none());
    drop(guard);
    assert!(lf2.try_lock().is_some());
}
