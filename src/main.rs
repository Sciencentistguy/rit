#![allow(dead_code)]

mod database;
mod interface;
mod storable;
mod util;

extern crate color_eyre as colour_eyre;

use std::path::Path;
use std::path::PathBuf;

use crate::storable::Blob;
use crate::storable::Storable;
use clap::Parser;
pub use colour_eyre::Result;
use once_cell::sync::Lazy;

use interface::*;

static ARGS: Lazy<Opt> = Lazy::new(Opt::parse);
static ROOT: Lazy<PathBuf> = Lazy::new(|| match &ARGS.path {
    Some(x) => x.clone(),
    None => std::env::current_dir().expect("Process has no directory :thonk:"),
});

fn main() -> Result<()> {
    colour_eyre::install().unwrap();
    Lazy::force(&ARGS);

    match ARGS.command {
        Command::Init => init()?,
        Command::Commit => commit()?,
    }
    Ok(())
}

fn init() -> Result<()> {
    let dir = ROOT.join(".git");
    for d in ["objects", "refs"] {
        std::fs::create_dir_all(dir.join(d))?;
    }
    Ok(())
}

fn commit() -> Result<()> {
    let git_path = ROOT.join(".git");
    let db_path = git_path.join("objects");
    let wsp = Workspace::new(ROOT.as_path());
    let database = database::Database::new(db_path);
    for file in wsp.list_files()? {
        let filepath = ROOT.join(file);
        let data = std::fs::read(filepath)?;
        let blob = Blob::new(&data);
        database.store(&blob)?;
    }
    Ok(())
}

struct Workspace {
    path: PathBuf,
}

impl Workspace {
    const IGNORE: [&'static str; 1] = [".git"];

    fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().canonicalize().unwrap(),
        }
    }

    fn list_files(&self) -> Result<impl Iterator<Item = String>> {
        Ok(self.path.read_dir()?.filter_map(|x| {
            let x = x.ok()?;

            // TODO proper error handling
            let filename = match x.file_name().into_string() {
                Ok(x) => x,
                Err(e) => panic!("non-utf8 path name waa {:?}", e),
            };

            if Self::IGNORE.contains(&&*filename) {
                None
            } else {
                Some(filename)
            }
        }))
    }
}
