use bytes::BytesMut;
use reqwest::Url;
use serde::Deserialize;
use sha2::{
    Sha256,
    digest::{FixedOutput, Update},
};

use super::fetch::Error;

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

pub async fn fetch_releases(base_url: &str, repo: &str) -> Result<Vec<GitHubRelease>, Error> {
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

pub async fn fetch_asset(asset: &GitHubAsset) -> Result<Vec<u8>, Error> {
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
        return Err(Error::Status(response.status()));
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
