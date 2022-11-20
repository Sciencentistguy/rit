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
use color_eyre::eyre::{eyre, Context};
pub use color_eyre::Result;
use repo::diff::DiffMode;
use repo::status::StatusOutputMode;
use revision::Rev;
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

    let level = match ARGS.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        _ => Level::TRACE,
    };

    let subscriber = Subscriber::builder()
        .with_max_level(level)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Verbosity level: {} ({})", level, ARGS.verbose);

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

    if let Command::Init { branch_name } = &ARGS.command {
        Repo::init(&path, &branch_name)?;
        return Ok(());
    }

    let mut repo = Repo::open(path)?;

    match &ARGS.command {
        Command::Init { .. } => unreachable!("Init command is handled above"),

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

        #[allow(unused_variables)]
        Command::Branch {
            patterns,
            delete,
            list,
            force,
        } => {
            match patterns.as_slice() {
                [] => todo!("List branches"),

                [name] => {
                    // FIXME: don't explode on a just-inited repo
                    let head = repo
                        .read_head()?
                        .ok_or_else(|| eyre!("Repo does not have a HEAD"))?;

                    repo.create_branch(name, &head)?
                }
                [name, rev] => {
                    let rev = Rev::parse(rev)?
                        .resolve(&repo)?
                        .ok_or_else(|| eyre!("Provided revision does not exist: '{}'", rev))?;
                    repo.create_branch(name, &rev)?
                }
                _ => todo!("catch this with clap?"),
            }
        }
    };

    Ok(())
}
