//! Helper functions for downloading and compiling nearcore.
//! These functions are used in the integration tests that require running Near nodes.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

pub async fn clone_nearcore(nearcore_root: &Path, tag: &str) -> anyhow::Result<PathBuf> {
    let status = Command::new("git")
        .current_dir(nearcore_root)
        .args(["clone", "https://github.com/near/nearcore.git"])
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::Error::msg("Failed to clone nearcore"));
    }

    let expected_path = nearcore_root.join("nearcore");

    if !expected_path.exists() {
        return Err(anyhow::Error::msg(
            "nearcore repository not created in the expected location",
        ));
    }

    let status = Command::new("git")
        .current_dir(&expected_path)
        .args(["checkout", tag])
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::Error::msg(
            "Failed to checkout current nearcore version",
        ));
    }

    Ok(expected_path)
}

pub async fn build_neard(nearcore_repository: &Path) -> anyhow::Result<PathBuf> {
    let status = Command::new("make")
        .current_dir(nearcore_repository)
        .arg("neard")
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::Error::msg("Failed to build neard binary"));
    }

    let expected_path = nearcore_repository
        .join("target")
        .join("release")
        .join("neard");

    if !expected_path.exists() {
        return Err(anyhow::Error::msg(
            "neard binary not created in the expected location",
        ));
    }

    Ok(expected_path)
}

pub async fn create_localnet_configs(
    nearcore_root: &Path,
    neard_binary: &Path,
) -> anyhow::Result<()> {
    let nearcore_home = nearcore_root
        .to_str()
        .ok_or_else(|| anyhow::Error::msg("Corrupt neard_root path"))?;
    let status = Command::new(neard_binary)
        .args([
            "--home",
            nearcore_home,
            "localnet",
            "--non-validators",
            "1",
            "--validators",
            "1",
            "--shards",
            "1",
            "--archival-nodes",
        ])
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::Error::msg(
            "Failed to generate localnet nearcore configs",
        ));
    }

    Ok(())
}

pub async fn start_neard(
    neard_binary: &Path,
    neard_home: &Path,
) -> anyhow::Result<tokio::process::Child> {
    let neard_home = neard_home
        .to_str()
        .ok_or_else(|| anyhow::Error::msg("Corrupt neard_home path"))?;

    let child = Command::new(neard_binary)
        .args(["--home", neard_home, "run"])
        .stderr(Stdio::piped())
        .spawn()?;

    Ok(child)
}
