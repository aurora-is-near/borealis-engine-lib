use std::{collections::BTreeMap, fmt, str, time::Duration};

use near_jsonrpc_client::{
    JsonRpcClient, NEAR_TESTNET_RPC_URL,
    errors::{JsonRpcError, JsonRpcServerError},
    methods::query::{RpcQueryError, RpcQueryRequest},
};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use tokio::time::Instant;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GetVersionError<E> {
    #[error("Failed to query version: {0}")]
    Inner(E),
    #[error("Received unexpected response")]
    UnexpectedResponse,
    #[error("Failed to decode UTF-8 string")]
    Utf8Error(#[from] str::Utf8Error),
    #[error("Operation timed out: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),
}

impl GetVersionError<JsonRpcError<RpcQueryError>> {
    pub const fn out_of_range(&self) -> bool {
        matches!(
            self,
            Self::Inner(JsonRpcError::ServerError(JsonRpcServerError::HandlerError(
                RpcQueryError::UnknownBlock { .. },
            )))
        )
    }
}

pub async fn get(
    height: u64,
    mainnet: bool,
) -> Result<String, GetVersionError<JsonRpcError<RpcQueryError>>> {
    let url = if mainnet {
        "https://archival-rpc.mainnet.near.org"
    } else {
        NEAR_TESTNET_RPC_URL
    };
    let client = JsonRpcClient::connect(url);
    let request = serde_json::from_value::<RpcQueryRequest>(serde_json::json!({
        "request_type": "call_function",
        "block_id": height,
        "account_id": "aurora",
        "method_name": "get_version",
        "args_base64": "",
    }))
    .expect("Format query request");
    let result = tokio::time::timeout(Duration::from_secs(4), client.call(request))
        .await?
        .map_err(GetVersionError::Inner)?;
    match result.kind {
        QueryResponseKind::CallResult(r) => Ok(str::from_utf8(&r.result)?.trim_end().to_string()),
        _ => Err(GetVersionError::UnexpectedResponse),
    }
}

struct VersionRequest {
    last_response: Option<Instant>,
    backoff: Duration,
}

impl Default for VersionRequest {
    fn default() -> Self {
        VersionRequest {
            last_response: None,
            backoff: Self::DEFAULT_DELAY,
        }
    }
}

impl VersionRequest {
    const DEFAULT_DELAY: Duration = Duration::from_secs(2);
    const EXPONENT: u32 = 2;

    async fn run(
        &mut self,
        height: u64,
    ) -> Result<String, GetVersionError<JsonRpcError<RpcQueryError>>> {
        loop {
            if let Some(last) = self.last_response {
                tokio::time::sleep_until(last + self.backoff).await;
            } else {
                tokio::time::sleep(self.backoff).await;
            }
            let res = get(height, true).await;
            self.last_response = Some(Instant::now());
            println!("{} -> {:?}, {:?}", height, res, self.backoff);
            if res.is_ok() {
                self.backoff = Self::DEFAULT_DELAY;
            } else if let Err(err) = &res {
                println!("{err}");
                if err.to_string().contains("rate limit") {
                    self.backoff = (self.backoff * Self::EXPONENT).min(Duration::from_secs(60));
                    continue;
                }
            }
            break res;
        }
    }
}

#[derive(Clone)]
pub struct VersionMap {
    inaccurate: u64,
    inner: BTreeMap<u64, String>,
}

impl Default for VersionMap {
    fn default() -> Self {
        Self {
            inaccurate: 160_000_000,
            inner: BTreeMap::from([
                (134229098, "3.7.0".to_owned()),
                (143772514, "3.9.0".to_owned()),
                (154664694, "3.9.1".to_owned()),
                (159429079, "3.9.2".to_owned()),
            ]),
        }
    }
}

impl fmt::Display for VersionMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (height, version) in &self.inner {
            writeln!(f, "{height} -> {version}")?;
        }

        Ok(())
    }
}

impl VersionMap {
    pub fn version_at_height(&self, height: u64) -> Option<&str> {
        if height >= self.inaccurate {
            return None;
        }
        if height < *self.inner.iter().next()?.0 {
            return Some("3.6.4");
        }
        self.inner
            .iter()
            .take_while(|(x, _)| **x <= height)
            .last()
            .map(|(_, x)| x.as_ref())
    }

    pub async fn populate(&mut self) {
        self.inaccurate = u64::MAX;
        let mut req = VersionRequest::default();
        let (mut initial_height, mut current_version) =
            self.inner.last_key_value().map(|(h, v)| (*h, v)).unwrap();
        while let (next_height, Some(next_version)) =
            Self::populate_next(&mut req, initial_height, current_version).await
        {
            initial_height = next_height;
            current_version = &*self.inner.entry(next_height).or_insert(next_version);
        }
    }

    async fn populate_next(
        req: &mut VersionRequest,
        initial_height: u64,
        current_version: &str,
    ) -> (u64, Option<String>) {
        // initial step is 2^15 blocks,
        // will return back if overrun
        let mut step = 15;

        // the supposed next version, it is not final, might be overrun
        // `None` means out of range
        let mut next_version = loop {
            match req.run(initial_height + (1 << step)).await {
                Ok(version) if version.ne(current_version) => {
                    break Some(version);
                }
                Err(err) if err.out_of_range() => {
                    break None;
                }
                Ok(_) => {
                    // go further
                    step += 1;
                }
                Err(_) => {
                    // TODO(vlad): limit retry
                }
            }
        };

        step -= 1;
        let mut offset = 1 << step;
        let mut overrun;
        loop {
            match req.run(initial_height + offset).await {
                Ok(version) if version.ne(current_version) => {
                    next_version = Some(version);
                    overrun = true;
                }
                Err(err) if err.out_of_range() => {
                    next_version = None;
                    overrun = true;
                }
                Ok(_) => {
                    overrun = false;
                }
                Err(_) => {
                    continue;
                }
            }
            if step == 0 {
                if !overrun {
                    offset += 1;
                }
                break;
            } else {
                step -= 1;
                if overrun {
                    offset -= 1 << step;
                } else {
                    offset += 1 << step;
                }
            }
        }

        (initial_height + offset, next_version)
    }
}

#[cfg(test)]
mod tests_version_map {
    use super::{VersionMap, VersionRequest};

    #[test]
    fn version_map() {
        let map = VersionMap::default();

        assert_eq!(map.version_at_height(134229097), Some("3.6.4"));
        assert_eq!(map.version_at_height(134229098), Some("3.7.0"));
        assert_eq!(map.version_at_height(134229099), Some("3.7.0"));

        assert_eq!(map.version_at_height(143772513), Some("3.7.0"));
        assert_eq!(map.version_at_height(143772514), Some("3.9.0"));
        assert_eq!(map.version_at_height(143772515), Some("3.9.0"));

        assert_eq!(map.version_at_height(154664693), Some("3.9.0"));
        assert_eq!(map.version_at_height(154664694), Some("3.9.1"));
        assert_eq!(map.version_at_height(154664695), Some("3.9.1"));

        assert_eq!(map.version_at_height(159429078), Some("3.9.1"));
        assert_eq!(map.version_at_height(159429079), Some("3.9.2"));
        assert_eq!(map.version_at_height(159429080), Some("3.9.2"));
    }

    #[tokio::test]
    async fn out_of_range() {
        let mut req = VersionRequest::default();
        match req.run(200_000_000).await {
            Ok(_) => {}
            Err(err) if err.out_of_range() => {}
            Err(err) => panic!("unexpected error: {err}"),
        }
    }

    #[ignore = "rate limit for RPC is too strict, the test takes too long"]
    #[tokio::test]
    async fn version_map_rpc() {
        let map = VersionMap::default();
        let mut req = VersionRequest::default();

        for height in [
            (134229097..).take(3),
            (143772513..).take(3),
            (154664692..).take(3),
            (159429077..).take(3),
        ]
        .into_iter()
        .flatten()
        {
            let actual = req.run(height).await.unwrap();
            let expected = map.version_at_height(height).unwrap();
            assert_eq!(actual, expected, "{height}");
        }
    }

    #[ignore = "rate limit for RPC is too strict, the test takes too long"]
    #[tokio::test]
    async fn version_map_populate() {
        let mut map = VersionMap::default();
        map.populate().await;
        println!("{map}");
    }
}
