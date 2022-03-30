use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    Init,
    Commit {
        #[clap(short, long)]
        message: Option<String>,
    },
}

#[derive(Debug, Parser)]
pub struct Opt {
    #[clap(subcommand)]
    pub command: Command,

    #[clap(short, long)]
    pub verbose: bool,

    /// The path to be used.
    #[clap(short)]
    pub path: Option<PathBuf>,
}
