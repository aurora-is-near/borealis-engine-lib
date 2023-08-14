pub mod ansi_utils;
pub mod nearcore_utils;
pub mod parse_logs;
pub mod refiner_utils;
pub mod toml_utils;

/// Integration test that just checks refiner-app will start and connect to a Near network.
/// This test does not check if Aurora blocks are produced from the Near blocks.
/// This is a fairly heavy test because it downloads and compiles nearcore in order to
/// set up a local Near network for the refiner to connect to.
#[tokio::test]
async fn test_refiner_starts() {
    use std::path::PathBuf;

    let repository_root = refiner_utils::get_repository_root().await.unwrap();
    let nearcore_root = tempfile::tempdir().unwrap();
    let nearcore_version = refiner_utils::get_nearcore_version(&repository_root)
        .await
        .unwrap();

    // Compiler refiner
    let thread_local_path = repository_root.clone();
    let refiner_binary =
        tokio::spawn(async move { refiner_utils::compile_refiner(&thread_local_path).await });

    // Clone and build nearcore if missing or different version
    let thread_local_path: PathBuf = nearcore_root.path().into();
    let neard_binary = tokio::spawn(async move {
        let neard_path = nearcore_utils::neard_path().await;
        if matches!(&neard_path, Ok(path) if path.exists() && nearcore_utils::neard_version(path).await? == nearcore_version)
        {
            neard_path
        } else {
            let nearcore_repo =
                nearcore_utils::clone_nearcore(&thread_local_path, &nearcore_version).await?;
            nearcore_utils::build_neard(&nearcore_repo).await
        }
    });

    let refiner_binary = refiner_binary.await.unwrap().unwrap();
    let neard_binary = neard_binary.await.unwrap().unwrap();

    // Setup config files for nearcore localnet
    nearcore_utils::create_localnet_configs(nearcore_root.path(), &neard_binary)
        .await
        .unwrap();

    // Start validator node
    let neard_home = nearcore_root.path().join("node0");
    let mut neard_process = nearcore_utils::start_neard(&neard_binary, &neard_home)
        .await
        .unwrap();
    let neard_output = neard_process.stderr.take().unwrap();
    let (neard_shutdown_sender, neard_shutdown_rx) = tokio::sync::oneshot::channel();
    let neard_process = tokio::spawn(async move {
        tokio::select! {
            _ = neard_process.wait() => (),
            _ = neard_shutdown_rx => neard_process.kill().await.expect("Failed to shutdown neard"),
        }
    });

    // Start refiner
    let mut refiner_process =
        refiner_utils::start_refiner(&refiner_binary, &repository_root, nearcore_root.path())
            .await
            .unwrap();
    let refiner_output = refiner_process.stdout.take().unwrap();
    let (refiner_shutdown_sender, refiner_shutdown_rx) = tokio::sync::oneshot::channel();
    let (refiner_status_sender, refiner_status_rx) = tokio::sync::oneshot::channel();
    let refiner_process = tokio::spawn(async move {
        let status = tokio::select! {
            status = refiner_process.wait() => status.unwrap(),
            _ = refiner_shutdown_rx => {
                refiner_process.start_kill().expect("Failed to send kill signal to refiner");
                refiner_process.wait().await.unwrap()
            }
        };
        refiner_status_sender.send(status).ok();
    });

    // Wait for validator to produce at least 100 blocks
    parse_logs::wait_for_height(neard_output, 100)
        .await
        .unwrap();

    // Check refiner has received at least 100 blocks
    tokio::select! {
        result = parse_logs::wait_for_height(refiner_output, 100) => result.unwrap(),
        status = refiner_status_rx => {
            panic!("Unexpected refiner crash: {status:?}");
        }
    };

    // Shutdown refiner
    refiner_shutdown_sender.send(()).unwrap();
    refiner_process.await.unwrap();

    // Shutdown validator
    neard_shutdown_sender.send(()).unwrap();
    neard_process.await.unwrap();
}
