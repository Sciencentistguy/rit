use camino::Utf8PathBuf;
use clap::ArgAction;
use clap::Parser;
use clap::Subcommand;

use crate::digest::Digest;

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Create an empty Git repository, or reinitialize an existing one
    Init {
        #[clap(short, long = "initial-branch", default_value("master"))]
        branch_name: String,
    },

    /// Record changes to the repository
    Commit {
        #[clap(short, long, env = "RIT_COMMIT_MESSAGE")]
        message: Option<String>,
    },

    /// Add file contents to the index
    Add {
        #[clap(env = "RIT_ADD_PATH", num_args(0..))]
        paths: Vec<Utf8PathBuf>,
    },

    /// Provide content or type and size information for repository objects
    #[clap(subcommand)]
    CatFile(CatFile),

    /// Show the working tree status
    Status {
        /// Display status in porcelain format.
        #[clap(long)]
        porcelain: bool,

        /// Display status in long format. This is the default behaviour if no flags are given.
        #[clap(long, conflicts_with = "porcelain")]
        long: bool,
    },

    /// Show changes between commits, commit and working tree, etc
    Diff {
        #[clap(long)]
        cached: bool,
    },

    /// Equivalent to `jit/show_head.rb`
    ShowHead { oid: Option<Digest> },

    /// List, create, or delete branches
    Branch {
        #[clap(num_args(1..))]
        patterns: Vec<String>,
        #[clap(short = 'd', long)]
        delete: bool,
        #[clap(short, long)]
        list: bool,
        #[clap(short, long)]
        force: bool,
    },
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
        object_ref: String,
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

    /// How verbose the tracing output should be. Multiple flags increase verbosity. Note that this
    /// is overeridden by the `RUST_LOG` environment variable.
    #[clap(short, long, action(ArgAction::Count))]
    pub verbose: u8,

    /// The path to be used.
    #[clap(short)]
    pub path: Option<Utf8PathBuf>,
}
