use aurora_refiner_types::source_config::ContractSource;
use engine_standalone_storage::Storage;

use crate::contract;

#[tokio::test]
async fn scenario() {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive("aurora_standalone_engine::contract=debug".parse().unwrap())
        .from_env()
        .unwrap();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .unwrap_or_default();

    let engine_account_id = "aurora".parse().unwrap();
    let storage_path = tempfile::tempdir().unwrap();

    // storage for sequential application
    let mut storage =
        Storage::open_ensure_account_id(storage_path.path(), &engine_account_id).unwrap();

    // the storage obtained by `Storage::share` is the same underlying db, but independent WASM contract
    // this is handy for handling RPC (EthCall and tracing)
    let mut storage_for_tracing = storage.share();

    // at the very begging fetch all contracts from the remote
    contract::fetch_all(&storage, &ContractSource::Mock)
        .await
        .unwrap();

    // during sequential application (reindex) we apply contract version at the given height
    // this makes association between block height and version
    let height_to_version_map = [
        (100_000_000, "3.7.0"),
        (110_000_000, "3.9.0"),
        (120_000_000, "3.9.1"),
    ];
    for (height, version) in height_to_version_map {
        contract::apply(&mut storage, height, 0, Some(version)).unwrap();
        let actual_version = storage.runner_mut().get_version().unwrap();
        assert_eq!(actual_version.trim_end(), version);
    }

    // during handling RPC, we should not specify version, only block height
    // the storage should know already the version at the height,
    // because reindex should have done before serving RPC
    assert!(!storage_for_tracing.runner_mut().initialized());

    // regular case
    contract::apply(&mut storage_for_tracing, 100_100_000, 0, None).unwrap();
    let actual_version = storage_for_tracing.runner_mut().get_version().unwrap();
    assert_eq!(actual_version.trim_end(), "3.7.0");

    // edge case
    contract::apply(&mut storage_for_tracing, 100_000_000, 0, None).unwrap();
    let actual_version = storage_for_tracing.runner_mut().get_version().unwrap();
    assert_eq!(actual_version.trim_end(), "3.7.0");

    // edge case, not found
    let err = contract::apply(&mut storage_for_tracing, 99_999_999, 0, None).unwrap_err();
    assert!(matches!(
        err,
        contract::ContractApplyError::NotFound {
            height: 99_999_999,
            pos: 0
        }
    ));

    // regular case
    contract::apply(&mut storage_for_tracing, 110_000_001, 0, None).unwrap();
    let actual_version = storage_for_tracing.runner_mut().get_version().unwrap();
    assert_eq!(actual_version.trim_end(), "3.9.0");

    // update stream
    let stream = futures::stream::iter(Some((
        "3.9.2".to_owned(),
        contract::bundled::CONTRACT_3_9_2.to_vec(),
    )));
    // could update asynchronously
    let update_task = tokio::spawn({
        let storage = storage.share();
        async move {
            contract::update(stream, &storage).await;
        }
    });

    // we don't really need async for the scenario
    update_task.await.unwrap();

    // this will be triggered by sequential block application and the contract must be already in the database
    // so we must update the contract on borealis first and then update the contract on-chain
    contract::apply(&mut storage, 130_000_000, 0, Some("3.9.2")).unwrap();
    let actual_version = storage.runner_mut().get_version().unwrap();
    assert_eq!(actual_version.trim_end(), "3.9.2");
}
