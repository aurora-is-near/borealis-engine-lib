mod cli;
mod config;
mod conversion;
mod input;
mod socket;
mod store;
use anyhow::anyhow;
use std::{borrow::Cow, fs, path::Path};

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

#[actix::main]
async fn main() -> anyhow::Result<()> {
    setup_logs();

    let args: Cli = Cli::parse();

    let config_path = args.config_path.as_deref().unwrap_or("default_config.json");
    let config: config::Config = {
        let file = fs::File::open(config_path)?;
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| anyhow!("Cannot parse config, reason: {e}"))?
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
            let engine_path = Path::new(&config.refiner.engine_path);

            fs::create_dir_all(engine_path).map_err(|v| {
                anyhow!("Unable to create or open directory {engine_path:?}, reason: {v}")
            })?;

            aurora_refiner_lib::storage::init_storage(
                engine_path.to_path_buf(),
                config.refiner.engine_account_id.clone(),
                config.refiner.chain_id,
            );

            let tx_tracker_path = config.refiner.tx_tracker_path.as_ref().map_or_else(
                || Cow::Owned(engine_path.join("tx_tracker")),
                |path| Cow::Borrowed(Path::new(path)),
            );

            let ctx = aurora_standalone_engine::EngineContext::new(
                engine_path,
                config.refiner.engine_account_id,
                config.refiner.chain_id,
            )
            .unwrap();

            let socket_storage = ctx.storage.clone();

            // create a broadcast channel for sending a stop signal
            let (tx, mut rx1) = tokio::sync::broadcast::channel(1);
            let mut rx2 = tx.subscribe();

            tokio::join!(
                // listen to ctrl-c for shutdown
                async {
                    tokio::signal::ctrl_c()
                        .await
                        .expect("failed to listen for event");
                    tx.send(()).expect("failed to propagate stop signal");
                },
                // Run socket server
                async {
                    if let Some(socket_config) = config.socket_server {
                        socket::start_socket_server(
                            socket_storage,
                            Path::new(&socket_config.path),
                            &mut rx1,
                        )
                        .await
                    }
                },
                // Run Refiner
                aurora_refiner_lib::run_refiner::<&Path, ()>(
                    ctx,
                    config.refiner.chain_id,
                    tx_tracker_path.as_ref(),
                    input_stream,
                    output_stream,
                    last_block,
                    &mut rx2,
                ),
            );
        }
    }

    Ok(())
}
