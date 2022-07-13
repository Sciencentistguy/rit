#[cfg(test)]
mod test;

mod digest;
mod filemode;
mod interface;
mod lock;
mod repo;
mod storable;
mod util;

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

    let mut repo = match ARGS.path {
        Some(ref path) => Repo::open(path.canonicalize()?),
        None => Repo::open(std::env::current_dir()?.canonicalize()?),
    };

    match &ARGS.command {
        Command::Init => repo.init()?,
        Command::Commit { message } => {
            let commit_id = repo.commit(
                message
                    .as_deref()
                    .expect("Using an editor for commit message is currently unimplemented"),
            )?;
            println!("Created commit {}", commit_id.to_hex())
        }
        Command::Add { path } => {
            repo.add(path)?;
        }
    }
    Ok(())
}
