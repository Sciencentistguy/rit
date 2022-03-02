use std::path::PathBuf;
use std::str::FromStr;

use clap::ArgEnum;
use clap::Parser;

#[derive(Clone, Copy, Debug, ArgEnum)]
pub enum Command {
    Init,
    Commit,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "init" => Ok(Self::Init),
            "commit" => Ok(Self::Commit),
            x => Err(x.to_string()),
        }
    }
}

#[derive(Debug, Parser)]
pub struct Opt {
    pub command: Command,

    #[clap(short, long)]
    pub verbose: bool,

    /// The path to be used.
    #[clap(short)]
    pub path: Option<PathBuf>,
}
