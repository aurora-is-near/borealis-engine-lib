use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use prometheus::{self, Encoder, TextEncoder};
use std::convert::Infallible;
use tracing::{error, info, warn};

use crate::metrics::{HTTP_PROMETHEUS_REQUESTS_COUNT, PROCESS_ID};

/// Handler for the Prometheus metrics endpoint.
async fn prometheus_handler(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    HTTP_PROMETHEUS_REQUESTS_COUNT.inc();

    let mut buffer = vec![];

    let encoder = TextEncoder::new();
    if let Err(err) = encoder.encode(&prometheus::gather(), &mut buffer) {
        error!("Failed to encode Prometheus metrics: {err}");
        return Ok(error_response!(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode Prometheus metrics: {err}")
        ));
    }

    String::from_utf8(buffer).map_or_else(
        |err| {
            error!("Failed to convert buffer to UTF-8 {err}");
            Ok(error_response!(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to convert buffer to UTF-8 {err}")
            ))
        },
        |text| Ok(success_response!(encoder.format_type(), text.into_bytes())),
    )
}

/// Starts the Prometheus service and listens for shutdown messages.
// Suppressing a pub(crate) enum and a pub(crate) type alias inside a private module, both caused by tokio::select! macro
#[allow(clippy::redundant_pub_crate)]
pub async fn run(
    prometheus_address: &str,
    mut shutdown_channel: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    PROCESS_ID.set(std::process::id() as i64);

    let listener = tokio::net::TcpListener::bind(prometheus_address).await?;

    let server = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();

    let mut shutdown_channel = std::pin::pin!(shutdown_channel.recv());

    info!("refiner-lib metrics started on {prometheus_address}");

    loop {
        tokio::select! {
            conn = listener.accept() => {
                match conn {
                    Ok((stream, _)) => {
                        let stream = hyper_util::rt::TokioIo::new(Box::pin(stream));
                        let connection = server.serve_connection(
                            stream,
                            service_fn(prometheus_handler)
                        );
                        let connection = graceful.watch(connection.into_owned());

                        tokio::spawn(async move {
                            if let Err(err) = connection.await {
                                error!("prometheus connection error: {err}");
                            }
                        });
                    },
                    Err(err) => {
                        warn!("Error accepting connection: {err}. Retry to accept a new connection in 1 second.");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
            _ = shutdown_channel.as_mut() => {
                info!("Ctrl-C or shutdown signal received, dropping Prometheus listener");
                drop(listener);
                break;
            }
        };
    }

    graceful.shutdown().await;

    info!("refiner-lib metrics service stopped");
    Ok(())
}
