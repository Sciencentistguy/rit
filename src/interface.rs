use camino::Utf8PathBuf;
use clap::Parser;
use clap::Subcommand;

use crate::digest::Digest;

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    Init,

    Commit {
        #[clap(short, long, env = "RIT_COMMIT_MESSAGE")]
        message: Option<String>,
    },

    Add {
        #[clap(env = "RIT_ADD_PATH", multiple_values = true)]
        path: Vec<Utf8PathBuf>,
    },

    #[clap(subcommand)]
    CatFile(CatFile),

    Status,
}

// FIXME: This is exposing the full names of the subcommands.
#[derive(Clone, Debug, Subcommand)]
pub enum CatFile {
    /// Exit with status `ExitCode::SUCCESS` if `object` exists and is a valid object. If
    /// `object` is of an invalid format, exit with status `ExitCode::FAILURE`, and print an
    /// error to stderr.
    #[clap(short_flag = 'e')]
    Exists {
        #[clap(value_name = "object")]
        object: Digest,
    },

    /// Pretty-print the contents of `object` based on its type
    #[clap(short_flag = 'p')]
    PrettyPrint {
        #[clap(value_name = "object")]
        object: Digest,
    },

    /// Print the type of `object` to stdout
    #[clap(short_flag = 't')]
    Type {
        #[clap(value_name = "object")]
        object: Digest,

        /// Permit the query of broken/corrupt objects of unknown type
        #[clap(long)]
        allow_unknown_type: bool,
    },

    /// Print the size of `object` to stdout
    #[clap(short_flag = 's')]
    Size {
        #[clap(value_name = "object")]
        object: Digest,

        /// Permit the query of broken/corrupt objects of unknown type
        #[clap(long)]
        allow_unknown_type: bool,
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
    pub path: Option<Utf8PathBuf>,
}
