//! Helper functions for downloading and compiling nearcore.
//! These functions are used in the integration tests that require running Near nodes.

use std::ffi::OsStr;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

pub async fn clone_nearcore(nearcore_root: &Path, tag: &str) -> anyhow::Result<PathBuf> {
    let status = Command::new("git")
        .current_dir(nearcore_root)
        .args([
            "clone",
            "https://github.com/near/nearcore.git",
            "--branch",
            tag,
        ])
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

    let to_path = neard_path().await?;

    create_dirs().await?;
    tokio::fs::copy(&expected_path, &to_path).await?;

    Ok(to_path)
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

pub async fn neard_path() -> anyhow::Result<PathBuf> {
    crate::refiner_utils::get_repository_root()
        .await
        .map(|dir| dir.join("target").join("release").join("neard"))
}

async fn create_dirs() -> anyhow::Result<()> {
    tokio::fs::create_dir_all(
        crate::refiner_utils::get_repository_root()
            .await?
            .join("target")
            .join("release"),
    )
    .await
    .map_err(Into::into)
}

pub async fn neard_version<P: AsRef<OsStr> + Send>(neard_path: P) -> anyhow::Result<String> {
    let output = Command::new(neard_path).args(["-V"]).output().await?;
    String::from_utf8(output.stdout)
        .map_err(Into::into)
        .and_then(parse_neard_version)
}

pub fn parse_neard_version(output: String) -> anyhow::Result<String> {
    output
        .split(" (")
        .find(|x| x.ends_with(')'))
        .and_then(|x| x.trim_end_matches(')').split_once(' '))
        .map(|(_, ver)| ver.to_string())
        .ok_or_else(|| anyhow::anyhow!("couldn't parce neard version"))
}

#[test]
fn test_parse_neard_version() {
    let output =
        "neard (release 1.35.0) (build 1.35.0-modified) (rustc 1.69.0) (protocol 62) (db 37)";

    assert_eq!(
        parse_neard_version(output.to_string()).as_deref().unwrap(),
        "1.35.0"
    );

    let output =
        "neard (release 1.35.0-rc.1) (build 1.35.0-modified) (rustc 1.69.0) (protocol 62) (db 37)";

    assert_eq!(
        parse_neard_version(output.to_string()).as_deref().unwrap(),
        "1.35.0-rc.1"
    );
}
