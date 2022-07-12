#[cfg(test)]
mod test;

mod digest;
mod interface;
mod lock;
mod repo;
mod storable;
mod util;
mod filemode;

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

    let repo = match ARGS.path {
        Some(ref path) => Repo::new(path.clone()),
        None => Repo::new(std::env::current_dir()?),
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
            todo!()
        }
    }
    Ok(())
}
