mod cli;
mod config;
mod conversion;
mod input;
mod socket;
mod store;
use anyhow::anyhow;
use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use cli::Cli;

use aurora_refiner_lib::signal_handlers;
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

            // Broadcast shutdown channel
            let (shutdown_tx, mut shutdown_rx_refiner) = tokio::sync::broadcast::channel(16);
            let shutdown_rx_input_stream = shutdown_tx.subscribe();
            let shutdown_rx_output_stream = shutdown_tx.subscribe();
            let mut shutdown_rx_socket = shutdown_tx.subscribe();

            // Build input stream
            let (input_stream, task_input_stream) = match &config.input_mode {
                config::InputMode::DataLake(config) => input::data_lake::get_near_data_lake_stream(
                    next_block,
                    config,
                    shutdown_rx_input_stream,
                ),
                config::InputMode::Nearcore(config) => input::nearcore::get_nearcore_stream(
                    next_block,
                    config,
                    shutdown_rx_input_stream,
                ),
            };

            // Build output stream
            let (output_stream, task_output_stream) = get_output_stream(
                total,
                config.output_storage.clone(),
                shutdown_rx_output_stream,
            );

            // Init storage
            let engine_path = Path::new(&config.refiner.engine_path);

            fs::create_dir_all(engine_path).map_err(|v| {
                anyhow!("Unable to create or open directory {engine_path:?}, reason: {v}")
            })?;

            aurora_refiner_lib::storage::init_storage(
                engine_path,
                &config.refiner.engine_account_id,
                config.refiner.chain_id,
            );

            let tx_tracker_path = config
                .refiner
                .tx_tracker_path
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| engine_path.join("tx_tracker"));

            let ctx = aurora_standalone_engine::EngineContext::new(
                engine_path,
                config.contract_wasm_code_path,
                config.refiner.engine_account_id,
                config.refiner.chain_id,
            )
            .map_err(|err| anyhow!("Failed to create engine context: {:?}", err))?;

            let socket_storage = ctx.storage.clone();
            let runner = ctx.runner.clone();

            let (signals_result, input_result, output_result, ..) = tokio::join!(
                // Handle all signals
                signal_handlers::handle_all_signals(shutdown_tx),
                // Wait for input stream to finish
                task_input_stream,
                // Wait for output stream to finish
                task_output_stream,
                // Run socket server
                async {
                    if let Some(socket_config) = config.socket_server {
                        socket::start_socket_server(
                            socket_storage,
                            runner,
                            Path::new(&socket_config.path),
                            &mut shutdown_rx_socket,
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
                    &mut shutdown_rx_refiner,
                ),
            );

            if let Err(err) = signals_result {
                tracing::error!("Signal handler failed: {:?}", err);
            }
            if let Err(err) = input_result {
                tracing::error!("Input stream failed: {:?}", err);
            }
            if let Err(err) = output_result {
                tracing::error!("Output stream failed: {:?}", err);
            }
        }
    }

    tracing::info!("refiner-app finished");
    Ok(())
}
