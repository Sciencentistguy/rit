#![allow(dead_code)]

mod database;
mod interface;
mod storable;
mod util;
mod workspace;

pub use color_eyre::Result;

use std::path::PathBuf;

use crate::interface::*;
use crate::storable::Blob;
use crate::storable::Storable;
use crate::workspace::Workspace;

use clap::Parser;
use once_cell::sync::Lazy;

static ARGS: Lazy<Opt> = Lazy::new(Opt::parse);
static ROOT: Lazy<PathBuf> = Lazy::new(|| match &ARGS.path {
    Some(x) => x.clone(),
    None => std::env::current_dir().expect("Process has no directory :thonk:"),
});

fn main() -> Result<()> {
    color_eyre::install().unwrap();
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
