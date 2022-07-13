use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    Init,
    Commit {
        #[clap(short, long, env = "RIT_COMMIT_MESSAGE")]
        message: Option<String>,
    },
    Add {
        #[clap(env = "RIT_ADD_PATH")]
        path: PathBuf,
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
