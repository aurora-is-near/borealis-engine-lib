// Fetch contract from github releases

use std::sync::RwLock;

use engine_standalone_storage::Storage;
use thiserror::Error;

use aurora_refiner_types::source_config::ContractSource;

use crate::storage_ext;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("Network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("HTTP status error: {0}")]
    Status(reqwest::StatusCode),
    #[error("Deserialize error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub struct ContractKey {
    pub version: String,
    pub height: u64,
    pub pos: u16,
}

pub async fn all(storage: &RwLock<Storage>, source: &ContractSource) -> Result<(), FetchError> {
    match source {
        ContractSource::Github { base_url, repo } => {
            let releases = github::fetch_releases(base_url, repo).await?;
            for release in releases {
                if let Some(asset) = release
                    .assets
                    .into_iter()
                    .find(|x| x.name == "aurora-compat.wasm")
                {
                    let code = github::fetch_asset(&asset).await?;
                    tracing::info!(version = release.tag_name, "Fetched contract");
                    let storage = storage.read().expect("storage must not panic");
                    if let Err(err) =
                        storage_ext::store_contract_by_version(&storage, &release.tag_name, &code)
                    {
                        tracing::error!(
                            version = &release.tag_name,
                            err = format!("{err:?}"),
                            "Failed to store contract in storage"
                        );
                    } else {
                        tracing::info!(version = &release.tag_name, "Stored contract in storage");
                    }
                } else {
                    tracing::warn!(
                        version = release.tag_name,
                        "No aurora-compat.wasm asset found"
                    );
                    continue;
                }
            }
        }
    }

    Ok(())
}

/// the source must be TLS protected
pub async fn fetch_and_store_contract(
    storage: &RwLock<Storage>,
    link: &ContractSource,
    version: &str,
    height: u64,
    pos: u16,
) -> Option<Vec<u8>> {
    match network_fetch(version, link).await {
        Ok(code) => {
            tracing::info!(version = version, "Fetched contract");
            let storage = storage.read().expect("storage must not panic");
            if let Err(err) = storage_ext::store_contract(&storage, height, pos, &code) {
                tracing::error!(
                    version = &version,
                    err = format!("{err:?}"),
                    "Failed to store contract in storage"
                );
            } else {
                tracing::info!(version = version, "Stored contract in storage");
            }
            Some(code)
        }
        Err(err) => {
            tracing::error!(
                version = version,
                err = format!("{err:?}"),
                "Failed to fetch contract from github"
            );
            None
        }
    }
}

// https://github.com/aurora-is-near/aurora-engine/releases/download/%version%/aurora-compat.wasm
async fn network_fetch(version: &str, link: &ContractSource) -> Result<Vec<u8>, FetchError> {
    match link {
        ContractSource::Github { base_url, repo } => {
            let releases = github::fetch_releases(base_url, repo).await?;
            if let Some(release) = releases.into_iter().find(|x| x.tag_name == version) {
                if let Some(asset) = release
                    .assets
                    .into_iter()
                    .find(|x| x.name == "aurora-compat.wasm")
                {
                    github::fetch_asset(&asset).await
                } else {
                    Err(FetchError::Status(reqwest::StatusCode::NOT_FOUND))
                }
            } else {
                Err(FetchError::Status(reqwest::StatusCode::NOT_FOUND))
            }
        }
    }
}

mod github {
    use bytes::BytesMut;
    use reqwest::Url;
    use serde::Deserialize;
    use sha2::{
        Sha256,
        digest::{FixedOutput, Update},
    };

    use super::FetchError;

    #[derive(Deserialize)]
    pub struct GitHubRelease {
        pub tag_name: String,
        pub assets: Vec<GitHubAsset>,
    }

    #[derive(Deserialize, Debug)]
    pub struct GitHubAsset {
        pub name: String,
        browser_download_url: Url,
        size: u64,
        digest: Option<String>,
    }

    pub async fn fetch_releases(
        base_url: &str,
        repo: &str,
    ) -> Result<Vec<GitHubRelease>, FetchError> {
        let link_index = format!("{base_url}/repos/{repo}/releases");
        let user_agent = concat!("borealis-engine-lib/", env!("CARGO_PKG_VERSION"));
        let client = reqwest::Client::builder().user_agent(user_agent).build()?;
        let resp = client
            .get(link_index)
            .header("Accept", "application/json")
            .send()
            .await?;
        let index = resp.text().await?;
        let releases = serde_json::from_str::<Vec<GitHubRelease>>(&index)?;
        Ok(releases)
    }

    pub async fn fetch_asset(asset: &GitHubAsset) -> Result<Vec<u8>, FetchError> {
        let user_agent = concat!("borealis-engine-lib/", env!("CARGO_PKG_VERSION"));
        let client = reqwest::Client::builder().user_agent(user_agent).build()?;
        if asset.size > 64 * 1024 * 1024 {
            panic!("Asset too large: {}", asset.size);
        }
        let mut response = client
            .get(asset.browser_download_url.clone())
            .header("Accept", "Application/octet-stream")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(FetchError::Status(response.status()));
        }
        let mut bytes = BytesMut::default();
        while let Some(chunk) = response.chunk().await? {
            bytes.extend_from_slice(&chunk);
        }
        let bytes = bytes.to_vec();
        if let Some(digest) = &asset.digest {
            let result = Sha256::default().chain(&bytes).finalize_fixed();
            let computed = hex::encode(result);
            if &computed != digest {
                panic!(
                    "Digest mismatch: expected {digest}, got {computed} for asset {}",
                    asset.name
                );
            }
        }
        Ok(bytes)
    }
}
