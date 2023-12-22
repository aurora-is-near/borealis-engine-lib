use std::io;
use std::path::Path;

use aurora_standalone_engine::{
    gas::{estimate_gas, EthCallRequest},
    tracing::lib::{trace_transaction, DebugTraceTransactionRequest},
};
use engine_standalone_storage::Storage;
use engine_standalone_tracing::types::call_tracer::SerializableCallFrame;
use serde_json::json;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

type SharedStorage = std::sync::Arc<tokio::sync::RwLock<Storage>>;

pub async fn start_socket_server(
    storage: SharedStorage,
    path: &Path,
    stop_signal: &mut tokio::sync::broadcast::Receiver<()>,
) {
    // Remove the old socket file if it exists
    if Path::new(path).exists() {
        std::fs::remove_file(path).expect("failed to remove socket file");
    }

    let sock = UnixListener::bind(path).expect("failed to open socket");

    loop {
        tokio::select! {
            _ = stop_signal.recv() => {
                break
            },
            Ok((mut stream, _)) = sock.accept() => {
                let storage = storage.clone();
                tokio::task::spawn(async move {
                    handle_conn(storage, &mut stream).await;
                });
            }
        }
    }

    std::fs::remove_file(path).expect("failed to remove socket file");
}

async fn handle_conn(storage: SharedStorage, stream: &mut UnixStream) {
    loop {
        if stream.readable().await.is_err() || stream.writable().await.is_err() {
            continue;
        }
        match wrapped_read(stream).await {
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                eprintln!("error reading from stream: {:?}", e);
                break;
            }
            Ok(data) if data.is_empty() => break,
            Ok(data) => {
                match serde_json::from_slice::<serde_json::Value>(&data) {
                    Ok(req) => {
                        let mut res = serde_json::Map::new();
                        res.insert(
                            "id".into(),
                            req.get("id").cloned().unwrap_or(serde_json::Value::Null),
                        );
                        res.insert(
                            "jsonrpc".into(),
                            req.get("jsonrpc")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null),
                        );

                        match handle_msg(storage.clone(), req).await {
                            Ok(v) => res.insert("result".into(), v),
                            Err(e) => res.insert(
                                "error".into(),
                                serde_json::to_value(e).unwrap_or_default(),
                            ),
                        };

                        let res_body = serde_json::to_vec(&res).unwrap_or_default();
                        if let Err(e) = wrapped_write(stream, &res_body).await {
                            eprintln!("error writing to stream: {:?}", e);
                        }
                    }
                    Err(e) => {
                        let res = json!({
                            "id": serde_json::Value::Null,
                            "jsonrpc": "2.0",
                            "error": JsonRpcError {
                                code: -32700,
                                message: "Parse error".into(),
                                data: Some(e.to_string()),
                            }
                        });
                        let res_body = serde_json::to_vec(&res).unwrap_or_default();
                        if let Err(e) = wrapped_write(stream, &res_body).await {
                            eprintln!("error writing to stream: {:?}", e);
                        }
                    }
                };
            }
        };
    }
    let _ = stream.shutdown().await;
}

async fn handle_msg(
    storage: SharedStorage,
    msg: serde_json::Value,
) -> Result<serde_json::Value, JsonRpcError<String>> {
    match msg
        .get("method")
        .ok_or(JsonRpcError {
            code: -32600,
            message: "Invalid Request".into(),
            data: Some("no method defined".into()),
        })?
        .as_str()
    {
        Some("eth_estimateGas") => handle_estimate_gas(storage, msg).await,
        Some("debug_traceTransaction") => handle_trace_transaction(storage, msg).await,
        _ => Err(JsonRpcError {
            code: -32601,
            message: "Method not found".into(),
            data: None,
        }),
    }
}

#[allow(clippy::significant_drop_tightening)]
async fn handle_estimate_gas(
    storage: SharedStorage,
    msg: serde_json::Value,
) -> Result<serde_json::Value, JsonRpcError<String>> {
    let req = EthCallRequest::from_json_value(msg).ok_or_else(|| invalid_params(None))?;
    let storage = storage.as_ref().read().await;
    let (res, _nonce) = estimate_gas(&storage, req, 0);
    match res {
        Err(_) => Err(internal_err(None)),
        Ok(res) => {
            // Add 33% buffer to avoid under-estimates.
            let estimate = res.gas_used.saturating_add(res.gas_used / 3);
            serde_json::to_value(estimate).map_err(|_| internal_err(Some("serialization failed")))
        }
    }
}

#[allow(clippy::significant_drop_tightening)]
async fn handle_trace_transaction(
    storage: SharedStorage,
    msg: serde_json::Value,
) -> Result<serde_json::Value, JsonRpcError<String>> {
    let req =
        DebugTraceTransactionRequest::from_json_value(msg).ok_or_else(|| invalid_params(None))?;
    let storage = storage.as_ref().read().await;
    let (res, _outcome) =
        trace_transaction(&storage, req.tx_hash).map_err(|_| internal_err(None))?;
    let mut traces = Vec::with_capacity(res.call_stack.len());
    for t in res.call_stack {
        let val = serde_json::to_value(SerializableCallFrame::from(t))
            .map_err(|_| internal_err(Some("serialization failed")))?;
        traces.push(val);
    }
    serde_json::to_value(traces).map_err(|_| internal_err(None))
}

fn internal_err(data: Option<&str>) -> JsonRpcError<String> {
    JsonRpcError {
        code: -32603,
        message: "Internal error".into(),
        data: data.map(|d| d.into()),
    }
}

fn invalid_params(data: Option<&str>) -> JsonRpcError<String> {
    JsonRpcError {
        code: -32602,
        message: "Invalid params".into(),
        data: data.map(|d| d.into()),
    }
}

/// As per the JSON-RPC 2.0 spec, section 5.1
/// https://www.jsonrpc.org/specification
#[derive(Debug, serde::Serialize)]
struct JsonRpcError<T> {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

/// Reads 4 bytes that indicate length of message and then the following message data.
pub async fn wrapped_read<R: AsyncRead + Unpin + Send>(reader: &mut R) -> io::Result<Vec<u8>> {
    let payload_len = reader.read_u32_le().await? as usize;
    let mut payload = vec![0; payload_len];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

/// Writes 4 bytes to indicate length of message followed by the message data.
pub async fn wrapped_write<W: AsyncWrite + Unpin + Send>(
    writer: &mut W,
    payload: &[u8],
) -> io::Result<()> {
    let payload_len = payload.len();
    writer.write_u32_le(payload_len as u32).await?;
    writer.write_all(payload).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_standalone_storage::json_snapshot::{self, types::JsonSnapshot};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    struct TestStorage {
        dir: tempfile::TempDir,
        storage: SharedStorage,
    }

    impl TestStorage {
        pub fn get(&self) -> SharedStorage {
            self.storage.clone()
        }

        pub fn close(self) {
            drop(self.storage);
            self.dir.close().unwrap();
        }
    }

    #[tokio::test]
    async fn test_stream_open_and_close() {
        let storage = init_storage();
        let server_storage = storage.get();
        let (mut client, mut handler) = UnixStream::pair().unwrap();
        tokio::try_join!(
            tokio::task::spawn(async move {
                client.shutdown().await.unwrap();
                let mut data = vec![0; 1];
                let res = client.read(&mut data).await.unwrap();
                assert_eq!(0, res);
            }),
            tokio::task::spawn(async move {
                handle_conn(server_storage, &mut handler).await;
            })
        )
        .unwrap();
        storage.close();
    }

    fn init_storage() -> TestStorage {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = Storage::open(dir.path().join("storage")).unwrap();

        // Initialize storage with data so that Engine can process transactions
        storage
            .set_engine_account_id(&"aurora".parse().unwrap())
            .unwrap();
        let snapshot =
            JsonSnapshot::load_from_file("src/tests/res/aurora_state_minimal.json").unwrap();
        let block_metadata = {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let random_seed = aurora_engine_sdk::keccak(&nanos.to_be_bytes());
            engine_standalone_storage::BlockMetadata {
                timestamp: aurora_engine_sdk::env::Timestamp::new(nanos as u64),
                random_seed,
            }
        };
        storage
            .set_block_data(
                Default::default(),
                snapshot.result.block_height + 1,
                &block_metadata,
            )
            .unwrap();
        json_snapshot::initialize_engine_state(&storage, snapshot).unwrap();

        TestStorage {
            dir,
            storage: Arc::new(RwLock::new(storage)),
        }
    }

    #[tokio::test]
    async fn test_parse_error() {
        let storage = init_storage();
        let server_storage = storage.get();
        let (mut client, mut handler) = UnixStream::pair().unwrap();
        tokio::try_join!(
            tokio::task::spawn(async move {
                client.writable().await.unwrap();

                wrapped_write(&mut client, b"foobar").await.unwrap();

                client.readable().await.unwrap();

                let data = wrapped_read(&mut client).await.unwrap();
                assert_eq!(118, data.len());

                let res: serde_json::Value = serde_json::from_slice(&data).unwrap();
                let want = json!({ "error": { "code": -32700, "data": "expected ident at line 1 column 2", "message": "Parse error" }, "id": null, "jsonrpc":"2.0" });
                assert_eq!(want, res);

                client.shutdown().await.unwrap();
                let data = wrapped_read(&mut client).await;
                assert_eq!(std::io::ErrorKind::UnexpectedEof, data.unwrap_err().kind());
            }),

            tokio::task::spawn(async move {
                handle_conn(server_storage, &mut handler).await;
            })
        ).unwrap();
        storage.close();
    }

    #[tokio::test]
    async fn test_trace_transaction() {
        let storage = init_storage();
        let server_storage = storage.get();
        let (mut client, mut handler) = UnixStream::pair().unwrap();
        tokio::try_join!(
            tokio::task::spawn(async move {
                client.writable().await.unwrap();

                // invalid params
                let req = json!({ "method": "debug_traceTransaction", "id": 1, "jsonrpc": "2.0" });
                let req_body = serde_json::to_vec(&req).unwrap();
                wrapped_write(&mut client, &req_body).await.unwrap();

                client.readable().await.unwrap();

                let data = wrapped_read(&mut client).await.unwrap();
                assert_eq!(75, data.len());

                let res: serde_json::Value = serde_json::from_slice(&data).unwrap();
                let want = json!({ "error": { "code": -32602, "message": "Invalid params" }, "id": 1, "jsonrpc": "2.0" });
                assert_eq!(want, res);

                // not found transaction
                let req = json!({ "method": "debug_traceTransaction", "params": ["0x2059dd53ecac9827faad14d364f9e04b1d5fe5b506e3acc886eff7a6f88a696a"], "id": 1, "jsonrpc": "2.0" });
                let req_body = serde_json::to_vec(&req).unwrap();
                wrapped_write(&mut client, &req_body).await.unwrap();

                let data = wrapped_read(&mut client).await.unwrap();
                assert_eq!(75, data.len());

                let res: serde_json::Value = serde_json::from_slice(&data).unwrap();
                let want = json!({ "error": { "code": -32603, "message": "Internal error" }, "id": 1, "jsonrpc": "2.0" });
                assert_eq!(want, res);

                client.shutdown().await.unwrap();
                let data = wrapped_read(&mut client).await;
                assert_eq!(std::io::ErrorKind::UnexpectedEof, data.unwrap_err().kind());
            }),

            tokio::task::spawn(async move {
                handle_conn(server_storage, &mut handler).await;
            })
        ).unwrap();
        storage.close();
    }

    #[tokio::test]
    async fn test_estimate_gas() {
        let storage = init_storage();
        let server_storage = storage.get();
        let (mut client, mut handler) = UnixStream::pair().unwrap();
        let input = std::fs::read_to_string("src/tests/res/test_estimate_gas_input.hex").unwrap();

        let req_task = tokio::task::spawn(async move {
            client.writable().await.unwrap();

            let req = json!({
                "id": 1,
                "jsonrpc": "2.0",
                "method": "eth_estimateGas",
                "params": [
                    {
                        "from": "0x1c76df114f0113e947d116d8cc2a9202921a2de0",
                        "data": input.trim(),
                    },
                ]
            });
            let req_body = serde_json::to_vec(&req).unwrap();
            wrapped_write(&mut client, &req_body).await.unwrap();

            client.readable().await.unwrap();
            let data = wrapped_read(&mut client).await.unwrap();
            let response: serde_json::Value = serde_json::from_slice(&data).unwrap();
            let expected = json!({ "result": 991508, "id": 1, "jsonrpc": "2.0" });
            assert_eq!(response, expected);

            client.shutdown().await.unwrap();
        });

        let server_task = tokio::task::spawn(async move {
            handle_conn(server_storage, &mut handler).await;
        });

        tokio::try_join!(req_task, server_task).unwrap();
        storage.close();
    }
}
