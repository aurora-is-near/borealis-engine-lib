use aurora_refiner_types::source_config::ContractSource;

use super::github;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("HTTP status error: {0}")]
    Status(reqwest::StatusCode),
    #[error("Deserialize error: {0}")]
    Serde(#[from] serde_json::Error),
}

// https://github.com/aurora-is-near/aurora-engine/releases/download/%version%/aurora-compat.wasm
pub async fn run(version: &str, link: &ContractSource) -> Result<Vec<u8>, Error> {
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
                    Err(Error::Status(reqwest::StatusCode::NOT_FOUND))
                }
            } else {
                Err(Error::Status(reqwest::StatusCode::NOT_FOUND))
            }
        }
    }
}
