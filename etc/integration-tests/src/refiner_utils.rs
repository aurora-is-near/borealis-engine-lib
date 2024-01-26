//! Helper functions for this repository.
//! These functions are used in the integration tests which require running
//! an instance of refiner-app.

use crate::toml_utils;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

pub async fn get_repository_root() -> anyhow::Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::Error::msg(
            "Command `git rev-parse --show-toplevel` failed",
        ));
    }

    let output = String::from_utf8(output.stdout)?;
    let path = PathBuf::from(output.trim());
    Ok(path)
}

pub async fn get_nearcore_version(repository_root: &Path) -> anyhow::Result<String> {
    let cargo_toml = tokio::fs::read_to_string(repository_root.join("Cargo.toml")).await?;
    let cargo_toml: toml::Table = toml::from_str(&cargo_toml)?;

    let nearcore_tag = toml_utils::toml_recursive_get(
        &cargo_toml,
        &["workspace", "dependencies", "near-indexer", "tag"],
    )?
    .as_str()
    .ok_or_else(|| anyhow::Error::msg("Expected nearcore tag to be string"))?;
    Ok(nearcore_tag.into())
}

pub async fn compile_refiner(repository_root: &Path) -> anyhow::Result<PathBuf> {
    #[cfg(not(feature = "ext-connector"))]
    let args = ["build", "-p", "aurora-refiner", "--release"];
    #[cfg(feature = "ext-connector")]
    let args = [
        "build",
        "-p",
        "aurora-refiner",
        "--features",
        "ext-connector",
        "--release",
    ];

    let status = Command::new("cargo")
        .current_dir(repository_root)
        .args(args)
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::Error::msg(
            "Failed to compile aurora-refiner package",
        ));
    }

    let expected_path = repository_root
        .join("target")
        .join("release")
        .join("aurora-refiner");

    if !expected_path.exists() {
        return Err(anyhow::Error::msg(
            "Refiner binary not created in the expected location",
        ));
    }

    Ok(expected_path)
}

pub async fn start_refiner(
    refiner_binary: &Path,
    repository_root: &Path,
    nearcore_root: &Path,
) -> anyhow::Result<tokio::process::Child> {
    let config = repository_root.join("nearcore_config.json");
    let config_path = config
        .to_str()
        .ok_or_else(|| anyhow::Error::msg("Corrupt refiner config path"))?;

    let child = Command::new(refiner_binary)
        .current_dir(nearcore_root)
        .args(["--config-path", config_path, "run"])
        .stdout(Stdio::piped())
        .spawn()?;

    Ok(child)
}
