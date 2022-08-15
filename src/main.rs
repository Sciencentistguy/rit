#![allow(dead_code)]

#[cfg(test)]
mod test;

mod blob;
mod cat_file;
mod commit;
mod digest;
mod filemode;
mod index;
mod interface;
mod lock;
mod repo;
mod storable;
mod tree;
mod util;

use camino::Utf8PathBuf;
use color_eyre::eyre::Context;
pub use color_eyre::Result;

use crate::digest::Digest;
use crate::interface::*;
use crate::repo::Repo;

use clap::Parser;
use once_cell::sync::Lazy;
use tracing_subscriber::prelude::*;

static ARGS: Lazy<Opt> = Lazy::new(Opt::parse);

fn main() -> Result<()> {
    color_eyre::install().unwrap();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    Lazy::force(&ARGS);

    let path = match ARGS.path {
        Some(ref path) => path
            .canonicalize_utf8()
            .wrap_err(format!("Failed to canonicalize path: '{}'", path))?,
        None => {
            let cwd = std::env::current_dir()?.canonicalize()?;
            Utf8PathBuf::from_path_buf(cwd).expect("Path must be valdi UTF-8")
        }
    };

    if matches!(ARGS.command, Command::Init) {
        Repo::init(&path)?;
        return Ok(());
    }

    let mut repo = Repo::open(path)?;

    match &ARGS.command {
        Command::Init => unreachable!("Init command is handled above"),

        Command::Commit { message } => {
            let commit_id = repo.commit(
                message
                    .as_deref()
                    .expect("Using an editor for commit message is currently unimplemented"),
            )?;
            println!("Created commit {}", commit_id.to_hex())
        }

        Command::Add { paths } => {
            if paths.is_empty() {
                repo.add_all()?
            } else {
                repo.add(paths)?
            }
        }

        Command::CatFile(args) => cat_file::handle(&mut repo, args)?,

        Command::Status { porcelain, long } => {
            let long = (!porcelain) || *long;
            repo.status(long)?
        }

        Command::ShowHead { oid } => repo.show_head(oid.clone())?,
    }

    Ok(())
}
