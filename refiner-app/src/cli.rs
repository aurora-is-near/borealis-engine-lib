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
        #[clap(short = 'n', long)]
        height: Option<u64>,
    },
}
