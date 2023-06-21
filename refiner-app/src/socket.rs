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
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Interest},
    net::{UnixListener, UnixStream},
};

type SharedStorage = std::sync::Arc<tokio::sync::RwLock<Storage>>;

pub async fn start_socket_server(storage: SharedStorage, path: &Path) {
    let sock = UnixListener::bind(path).expect("failed to open socket");

    loop {
        if let Ok((mut stream, _)) = sock.accept().await {
            let storage = storage.clone();
            tokio::task::spawn(async move {
                handle_conn(storage, &mut stream).await;
            });
        }
    }
}

async fn handle_conn(storage: SharedStorage, stream: &mut UnixStream) {
    match stream.ready(Interest::READABLE | Interest::WRITABLE).await {
        Ok(r) if r.is_writable() && r.is_readable() => (),
        r => {
            let _ = stream.shutdown().await;
            eprintln!("faulty stream: {:?}", r);
            return;
        }
    };
    loop {
        match wrapped_read(stream).await {
            Err(e) => {
                eprintln!("error reading from stream: {:?}", e);
                break;
            }
            Ok(data) if data.is_empty() => break,
            Ok(data) => {
                let req: serde_json::Value = match serde_json::from_slice(&data) {
                    Ok(v) => v,
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
                        continue;
                    }
                };
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
                    Err(e) => {
                        res.insert("error".into(), serde_json::to_value(e).unwrap_or_default())
                    }
                };

                let res_body = serde_json::to_vec(&res).unwrap_or_default();
                if let Err(e) = wrapped_write(stream, &res_body).await {
                    eprintln!("error writing to stream: {:?}", e);
                }
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

async fn handle_estimate_gas(
    storage: SharedStorage,
    msg: serde_json::Value,
) -> Result<serde_json::Value, JsonRpcError<String>> {
    let req = EthCallRequest::from_json_value(msg).ok_or_else(|| invalid_params(None))?;
    let storage = storage.as_ref().read().await;
    let (res, _nonce) = estimate_gas(&storage, req, 0);
    match res {
        Err(_) => Err(internal_err(None)),
        Ok(res) => serde_json::to_value(res.gas_used)
            .map_err(|_| internal_err(Some("serialization failed"))),
    }
}

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
pub async fn wrapped_read<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Vec<u8>> {
    let payload_len = reader.read_u32_le().await? as usize;
    let mut payload = vec![0; payload_len];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

/// Writes 4 bytes to indicate length of message followed by the message data.
pub async fn wrapped_write<W: AsyncWrite + Unpin>(
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

    #[tokio::test]
    async fn test_stream_open_and_close() {
        let db_dir = tempfile::tempdir().unwrap().path().join("storage");
        let storage = Storage::open(db_dir)
            .map(tokio::sync::RwLock::new)
            .map(std::sync::Arc::new)
            .unwrap();
        let (mut first, mut second) = UnixStream::pair().unwrap();
        let handler = tokio::task::spawn(async move { handle_conn(storage, &mut second).await });
        let client = tokio::task::spawn(async move {
            first.shutdown().await.unwrap();
            let mut data = vec![0; 10];
            let res = first.read(&mut data).await.unwrap();
            assert_eq!(0, res);
        });

        tokio::try_join!(handler, client).unwrap();
    }

    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn init_storage() -> SharedStorage {
        let db_dir = tempfile::tempdir().unwrap().path().join("storage");
        Storage::open(db_dir)
            .map(RwLock::new)
            .map(Arc::new)
            .unwrap()
    }

    #[tokio::test]
    async fn test_parse_error() {
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
                handle_conn(init_storage(), &mut handler).await;
            })
        ).unwrap();
    }

    #[tokio::test]
    async fn test_trace_transaction() {
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
                handle_conn(init_storage(), &mut handler).await;
            })
        ).unwrap();
    }
}
