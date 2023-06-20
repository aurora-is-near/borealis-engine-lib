mod cli;
mod config;
mod conversion;
mod input;
mod socket;
mod store;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use clap::Parser;
use cli::Cli;

use store::{get_output_stream, load_last_block_height};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

fn setup_logs() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::try_from_env("REFINER_LOG")
                .unwrap_or_else(|_| EnvFilter::default().add_directive("info".parse().unwrap())),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");
}

#[tokio::main]
async fn main() -> Result<(), tokio::io::Error> {
    setup_logs();

    let args: Cli = Cli::parse();

    let config_path = args.config_path.as_deref().unwrap_or("default_config.json");
    let config: config::Config = {
        let file = std::fs::File::open(config_path).unwrap();
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    };

    match args.command {
        cli::Command::Run { height, total } => {
            // Load last block
            let (last_block, next_block) = if let Some(height) = height {
                (height.checked_sub(1), height)
            } else {
                let last_block = load_last_block_height(&config.output_storage.path).await;
                let next_block = last_block.map(|x| x + 1).unwrap_or(0);
                (last_block, next_block)
            };

            // Build input stream
            let input_stream = match config.input_mode {
                config::InputMode::DataLake(config) => {
                    input::data_lake::get_near_data_lake_stream(next_block, &config)
                }
                config::InputMode::Nearcore(config) => {
                    input::nearcore::get_nearcore_stream(next_block, &config)
                }
            };

            // Build output stream
            let output_stream = get_output_stream(total, config.output_storage.clone());

            // Init storage
            aurora_refiner_lib::storage::init_storage(
                PathBuf::from(&config.refiner.engine_path),
                config.refiner.engine_account_id.parse().unwrap(),
                config.refiner.chain_id,
            );

            let engine_path = Path::new(&config.refiner.engine_path);
            let tx_tracker_path = match config.refiner.tx_tracker_path.as_ref() {
                Some(path) => Cow::Borrowed(Path::new(path)),
                None => Cow::Owned(engine_path.join("tx_tracker")),
            };

            // Run Refiner
            aurora_refiner_lib::run_refiner::<&Path, ()>(
                config.refiner.chain_id,
                engine_path,
                tx_tracker_path.as_ref(),
                config.refiner.engine_account_id.parse().unwrap(),
                input_stream,
                output_stream,
                last_block,
            )
            .await;
        }
    }

    Ok(())
}
