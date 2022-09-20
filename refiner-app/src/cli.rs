use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to config file
    #[clap(short, long)]
    pub config_path: Option<String>,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Execute Refiner
    Run {
        /// [Optional] Start refiner from specified height.
        /// If this value is passed, last height on disk will be ignored.
        /// Setting the height is only recommended for advanced users.
        #[clap(short = 'n', long)]
        height: Option<u64>,
        /// [Optional] Number of blocks to download. If not specified it will
        /// run no stop indexing the network in real time once in sync.
        #[clap(short, long)]
        total: Option<u64>,
    },
}
