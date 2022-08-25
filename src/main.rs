#![allow(dead_code)]

#[cfg(test)]
mod test;

mod blob;
mod cat_file;
mod commit;
mod diff;
mod digest;
mod filemode;
mod index;
mod interface;
mod repo;
mod revision;
mod storable;
mod tree;
mod util;

use camino::Utf8PathBuf;
use color_eyre::eyre::Context;
pub use color_eyre::Result;
use repo::diff::DiffMode;
use repo::status::StatusOutputMode;
use tracing::{info, Level};
use tracing_subscriber::fmt::Subscriber;
use tracing_subscriber::EnvFilter;

use crate::digest::Digest;
use crate::interface::*;
use crate::repo::Repo;

use clap::Parser;
use once_cell::sync::Lazy;

static ARGS: Lazy<Opt> = Lazy::new(Opt::parse);

fn main() -> Result<()> {
    color_eyre::install().unwrap();

    match ARGS.verbose {
        0 => {
            let subscriber = Subscriber::builder()
                .with_env_filter(EnvFilter::from_default_env())
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        1 => {
            let subscriber = Subscriber::builder()
                .with_env_filter(EnvFilter::from_default_env())
                .with_max_level(Level::INFO)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
            info!("Verbosity level: INFO (1)");
        }
        x => {
            let subscriber = Subscriber::builder()
                .with_env_filter(EnvFilter::from_default_env())
                .with_max_level(Level::TRACE)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
            info!("Verbosity level: TRACE ({x})");
        }
    };

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
            let mode = if !porcelain || *long {
                StatusOutputMode::Long
            } else {
                StatusOutputMode::Porcelain
            };
            repo.status(mode)?
        }

        Command::Diff { cached } => {
            let mode = if *cached {
                DiffMode::IndexHead
            } else {
                DiffMode::WorktreeIndex
            };
            repo.diff(mode)?
        }

        Command::ShowHead { oid } => repo.show_head(oid.clone())?,

        Command::Branch { name, delete } => repo.branch(name.as_deref(), *delete)?,
    };

    Ok(())
}
