use std::path::PathBuf;

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")");

#[derive(Debug, clap::Parser)]
pub struct ServerCommand {
    /// Path to the repo's tablejohn database.
    pub db: PathBuf,
    /// Path to the git repo.
    pub repo: Option<PathBuf>,
}

#[derive(Debug, clap::Parser)]
pub enum Command {
    Server(ServerCommand),
    Runner,
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
