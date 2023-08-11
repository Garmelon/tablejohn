use std::path::PathBuf;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")");

#[derive(Debug, clap::Parser)]
pub struct ServerCommand {
    /// Path to a tablejohn database.
    pub db: PathBuf,

    /// Path to a git repo.
    pub repo: Option<PathBuf>,

    /// Path to a bench repo.
    #[arg(long, short)]
    pub bench_repo: Option<PathBuf>,

    /// Open the UI in your browser.
    #[arg(long, short)]
    pub open: bool,

    /// Start one or more local workers for this server.
    #[arg(long, short, action = clap::ArgAction::Count)]
    pub local_worker: u8,
}

#[derive(Debug, clap::Parser)]
pub enum Command {
    /// Start a tablejohn server.
    Server(ServerCommand),
    /// Start a tablejohn worker.
    Worker,
    // TODO bench script command?
}

#[derive(Debug, clap::Parser)]
#[command(name = NAME, version = VERSION)]
pub struct Args {
    /// Path to the config file.
    #[arg(long, short)]
    pub config: Option<PathBuf>,

    /// Enable increasingly more verbose output
    #[arg(long, short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}
