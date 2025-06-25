# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.30.6-2.6.3] 2025-06-25

* fix: Correct sh syntax for the BEL gh action by [@aleksander-borodulya] in [#232]
* fix: Add missing serde flag to ChunkHeaderView by [@aleksander-borodulya] in [#231]
* fix: use selfhosted runners for rust tests by [@aleksuss] in [#230]

[#232]: https://github.com/aurora-is-near/borealis-engine-lib/pull/232
[#231]: https://github.com/aurora-is-near/borealis-engine-lib/pull/231
[#230]: https://github.com/aurora-is-near/borealis-engine-lib/pull/230

## [0.30.5-2.6.3] 2025-06-03

* fix: Rework `StreamerMessage` to `NEARBlock` conversion by [@aleksander-borodulya] in [#226]
* fix: Add benchmarks for `StreamerMessage` to `NEARBlock` conversion by [@aleksander-borodulya] in [#227]

[#226]: https://github.com/aurora-is-near/borealis-engine-lib/pull/226
[#227]: https://github.com/aurora-is-near/borealis-engine-lib/pull/227

## [0.30.4-2.6.3] 2025-05-16

### Changes

* chore: bump nearcore to 2.6.3 in [#225]

[#225]: https://github.com/aurora-is-near/borealis-engine-lib/pull/225

## [0.30.4-2.6.3-rc.2] 2025-05-15

### Changes

* chore: bump nearcore to 2.6.3-rc.2 in [#224]

[#224]: https://github.com/aurora-is-near/borealis-engine-lib/pull/224

## [0.30.4-2.6.3-rc.1] 2025-05-14

### Changes

* chore: bump nearcore to 2.6.3-rc.1 in [#223]

[#223]: https://github.com/aurora-is-near/borealis-engine-lib/pull/223

## [0.30.4-2.6.2] 2025-05-10

### Changes

* chore: bump nearcore to 2.6.2 in [#221]

[#221]: https://github.com/aurora-is-near/borealis-engine-lib/pull/221

## [0.30.4-2.6.2-rc.1] 2025-05-09

### Changes

* chore: bump nearcore to 2.6.2-rc.1 in [#220]

[#220]: https://github.com/aurora-is-near/borealis-engine-lib/pull/220

## [0.30.4-2.6.1-rc.1] 2025-05-05

### Changes

* chore: bump nearcore to 2.6.1-rc.1 in [#217]

[#217]: https://github.com/aurora-is-near/borealis-engine-lib/pull/217

## [0.30.4-2.6.1] 2025-05-01

### Changes

* chore: bump nearcore to 2.6.1 in [#216]

[#216]: https://github.com/aurora-is-near/borealis-engine-lib/pull/216

## [0.30.4-2.6.0] 2025-04-30

### Changes

* chore: bump nearcore to 2.6.0 in [#215]

[#215]: https://github.com/aurora-is-near/borealis-engine-lib/pull/215

## [0.30.4-2.6.0-rc.2] 2025-04-24

### Changes

* chore: bump nearcore to 2.6.0-rc.2 by [@alexander-borodulya] in [#211]

[#211]: https://github.com/aurora-is-near/borealis-engine-lib/pull/211

## [0.30.4-2.6.0-rc.1] 2025-04-08

### Changes

* chore: bump aurora-engine to 3.9.0 and nearcore to 2.6.0-rc.1 by [@alexander-borodulya] in [#208]

[#208]: https://github.com/aurora-is-near/borealis-engine-lib/pull/208

## [0.30.3-2.5.2] 2025-04-02

### Changes

* chore: bump nearcore to 2.5.2 by [@alexander-borodulya] in [#207]

[#207]: https://github.com/aurora-is-near/borealis-engine-lib/pull/207

## [0.30.3-2.5.1] 2025-03-08

### Fixes

* Temporary pin the `tempfile` crate to version 3.14 due to [issue 12944](https://github.com/near/nearcore/issues/12944#issuecomment-2707438357)

## [0.30.2-2.5.0-rc.3] 2025-02-27

## Changes

* Disable build for arm64 due to qemu failure by [@spilin] in [#199]
* Use input tag to checkout proper release by [@spilin] in [#201]

## Fixes

* chore: Bump nearcore to 2.5.0-rc.3 by [@alexander-borodulya] in [#200]
* Read proper package version as a source of the service version by [@alexander-borodulya] in [#202]

[#199]: https://github.com/aurora-is-near/borealis-engine-lib/pull/199
[#201]: https://github.com/aurora-is-near/borealis-engine-lib/pull/201
[#200]: https://github.com/aurora-is-near/borealis-engine-lib/pull/200
[#202]: https://github.com/aurora-is-near/borealis-engine-lib/pull/202

## [0.30.1-2.4.0] 2025-02-05

### Changes
* Support ghcr, arm64 and update runner by [@spilin] in [#191]
* Upgrade flow automation - Init workflows by [@alexander-borodulya] in [#192]
* chore: Bump aurora-engine to 3.8.0 by [@alexander-borodulya] in [#195]

### Fixes
* Print `determine_ft_on_transfer_recipient` error messages by [@alexander-borodulya] in [#193]

[#191]: https://github.com/aurora-is-near/borealis-engine-lib/pull/191
[#192]: https://github.com/aurora-is-near/borealis-engine-lib/pull/192
[#193]: https://github.com/aurora-is-near/borealis-engine-lib/pull/193
[#195]: https://github.com/aurora-is-near/borealis-engine-lib/pull/195

## [0.30.0-2.4.0] 2024-12-17

### Changes

* chore: bump nearcore to 2.4.0 by [@alexander-borodulya] in [#188]

### Fixes

* fix: use rand feature for near-primitives by [@aleksuss] in [#184]
* fix: processing deploy_erc20_token and mirror_erc20_token_callback by refiner by [@aleksuss] in [189]

[#184]: https://github.com/aurora-is-near/borealis-engine-lib/pull/184
[#188]: https://github.com/aurora-is-near/borealis-engine-lib/pull/188
[#189]: https://github.com/aurora-is-near/borealis-engine-lib/pull/189

## [0.29.0-2.3.0] 2024-11-07

### Changes

* chore: add test for check state init after processing batch transaction by [@aleksuss] in [#179]

### Fixes

* Fix handling mint transactions in aurora block processing by [@alexander-borodulya] in [#174]

[#174]: https://github.com/aurora-is-near/borealis-engine-lib/pull/174
[#179]: https://github.com/aurora-is-near/borealis-engine-lib/pull/179

## [0.28.2-2.3.0] 2024-10-31

### Changes
* chore: bump aurora-engine to 3.6.3 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/147
* chore(deps): bump rustls from 0.21.10 to 0.21.11 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/146
* chore(deps): bump curve25519-dalek from 4.1.2 to 4.1.3 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/151
* Chore: Update toolchain and nearcore to 2.1.0-rc.1 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/159
* chore: bump nearcore to 2.3.0 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/177

### Fixes

* fix: use engine account id from storage by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/169

## [0.28.1-1.39.1] 2024-04-19

### Changes
* Moving refiner by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/7
* Failing transactions should not be indexed by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/8
* fix(refiner): Filter all tx that fail on NEAR runtime by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/10
* test(refiner): Test blocks with aurora txs on it by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/9
* feat(app): Refiner app by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/11
* feat(cli): Allow starting from specified height by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/12
* Fix(engine): Skip transactions that failed on NEAR before even parsing that transaction by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/13
* fix(storage): Initialize engine storage properly by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/14
* fix: Failing NEAR tx are not reported as error by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/15
* Update engine dependency to version 2.7.0 by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/16
* Remove private deps and add actix as main runtime by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/17
* [feat]: Add total blocks to download optionally by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/18
* fix(hacken-low-2): Unused dependency by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/19
* External accessibility miscellanea by @Casuso in https://github.com/aurora-is-near/borealis-engine-lib/pull/21
* Chore: Update Aurora Engine to version 2.8.0 by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/23
* Chore: Avoid duplicating dependency versions using workspace inheritance by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/25
* chore: update some dependencies and re-export errors module by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/26
* Chore: Update to aurora-engine version 2.8.1 by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/27
* chore(deps): bump secp256k1 from 0.24.1 to 0.24.2 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/28
* Chore: use near-indexer from 1.30 release by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/29
* Fix(types): Use u64 for gas a nonce types by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/30
* fix: Populate contract address field when a new contract is deployed by @mfornet in https://github.com/aurora-is-near/borealis-engine-lib/pull/22
* Add test for processing block at height 81206675 by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/32
* Fix(refiner): Atomically update last block file by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/34
* Feat(refiner): Track the transaction hash the caused each receipt to be created by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/31
* Fix(refiner): Check Near and Engine outcomes match by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/35
* Feat(engine): Alchemy tracing format by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/36
* Fix(refiner-types): Aurora Block serialization backwards compatibility by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/37
* Fix(engine-tracing): Public fields on TransactionContext by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/38
* chore(deps): bump tokio from 1.23.0 to 1.24.1 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/39
* Nonce_and_address_from_state by @Casuso in https://github.com/aurora-is-near/borealis-engine-lib/pull/40
* Skip block test by @Casuso in https://github.com/aurora-is-near/borealis-engine-lib/pull/41
* Test block before genesis by @Casuso in https://github.com/aurora-is-near/borealis-engine-lib/pull/42
* chore(deps): bump tokio from 1.24.1 to 1.25.0 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/43
* Fix: refiner can process receipts with no actions by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/45
* Fix: only prune tx tracker DB irregularly by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/46
* Fix: configuring the transaction tracker path is optional by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/48
* Fix: ignore whitespace in last block file by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/49
* Feat(NEARBlock): Implement `Clone`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/50
* Chore: update Engine and Near dependencies (support DelegateAction) by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/52
* Build(deps): Update all dependencies; bump version. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/58
* Chore: upgrade near-lake-framework to version 0.7 by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/63
* chore(deps): bump h2 from 0.3.16 to 0.3.17 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/60
* Fix: if the execution outcome is a failure then it applies to the entire batch by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/64
* Chore: Update Cargo.toml to current release version by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/65
* chore: update aurora-engine up to 2.9.1 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/67
* Release 0.20.0 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/68
* Feat: use generic modexp impl in borealis engine library by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/71
* Add a unix socket server for handling RPC calls by @SamuelSarle in https://github.com/aurora-is-near/borealis-engine-lib/pull/73
* Feat: Validate `engine_path` before passing it to `init_storage`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/76
* Feat: Implement validation of Near account ID in config. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/77
* Refactor: Remove wrapper type `ValidatedAccountId`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/78
* chore: update aurora-engine up to 2.10 by @vimpunk in https://github.com/aurora-is-near/borealis-engine-lib/pull/79
* chore: update aurora-engine to 2.10.1 by @vimpunk in https://github.com/aurora-is-near/borealis-engine-lib/pull/82
* fix: udpate cargo.lock by @vimpunk in https://github.com/aurora-is-near/borealis-engine-lib/pull/83
* Fix: use actix runtime in refiner-app by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/86
* chore: update and remove unused dependencies by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/87
* chore: bump aurora up to 2.10.2 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/90
* chore: speed up ci flow by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/91
* Fix: check diff before commiting it to storage by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/92
* Feat: Integrate Hashchain into Refiner by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/95
* Chore(deps): Update Aurora Engine to commit aa20b0c by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/97
* Remove socket file if it exists, before establishing connection by @spilin in https://github.com/aurora-is-near/borealis-engine-lib/pull/98
* Chore: update `aurora-engine` to 3.1.0 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/99
* feat: bump aurora-engine to 3.2.0 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/102
* Feat: re-export `aurora-engine` and `nearcore` dependencies by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/103
* fix: pass ext-connector feature to dependencies by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/105
* chore: bump aurora-engine to 3.3.1 and remove re-exports by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/107
* Fix(engine): Compute action_hash for each transaction by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/110
* feat: add label with version to prometheus metrics by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/109
* Feat(refiner): Include EVM logs for bridge transactions by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/111
* Build(deps): Bump `aurora-engine` version `7fff9ff` => `3.4.0` by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/113
* chore(deps): bump openssl from 0.10.57 to 0.10.60 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/112
* Build(deps): Bump `aurora-engine` version `3.4.0` => `3.5.0` by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/114
* Fix(rpc): Estimate gas properly estimates contract deployments by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/118
* chore: bump version up to 0.25.7 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/119
* chore: bump rust to 1.74.0 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/121
* chore(deps): bump shlex from 1.2.0 to 1.3.0 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/123
* chore(deps): bump aurora-engine to 3.6.0 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/126
* fix: use engine account id from storage in eth_call by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/127
* Build and push docker container on release by @spilin in https://github.com/aurora-is-near/borealis-engine-lib/pull/137
* chore: bump aurora-engine to 3.6.2 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/138
* chore: add a dockerhub link to readme by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/140
* chore(deps): bump h2 from 0.3.24 to 0.3.26 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/143
* chore: bump nearcore to 1.39.0 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/144

## [v0.10.0] 

[Unreleased]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.6-2.6.3...main
[0.30.6-2.6.3]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.5-2.6.3...0.30.6-2.6.3
[0.30.5-2.6.3]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.3...0.30.5-2.6.3
[0.30.4-2.6.3]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.3-rc.2...0.30.4-2.6.3
[0.30.4-2.6.3-rc.2]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.3-rc.1...0.30.4-2.6.3-rc.2
[0.30.4-2.6.3-rc.1]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.2...0.30.4-2.6.3-rc.1
[0.30.4-2.6.2]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.2-rc.1...0.30.4-2.6.2
[0.30.4-2.6.2-rc.1]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.1-rc.1...0.30.4-2.6.2-rc.1
[0.30.4-2.6.1-rc.1]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.1...0.30.4-2.6.1-rc.1
[0.30.4-2.6.1]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.0...0.30.4-2.6.1
[0.30.4-2.6.0]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.0-rc.2...0.30.4-2.6.0
[0.30.4-2.6.0-rc.2]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.6.0-rc.1...0.30.4-2.6.0-rc.2
[0.30.4-2.6.0-rc.1]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.4-2.5.2...0.30.4-2.6.0-rc.1
[0.30.4-2.5.2]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.3-2.5.2...0.30.4-2.5.2
[0.30.3-2.5.2]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.3-2.5.1...0.30.3-2.5.2
[0.30.3-2.5.1]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.2-2.5.0...0.30.3-2.5.1
[0.30.2-2.5.0]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.2-2.5.0-rc.3...0.30.2-2.5.0
[0.30.2-2.5.0-rc.3]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.1-2.4.0...0.30.2-2.5.0-rc.3
[0.30.1-2.4.0]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.30.0-2.4.0...0.30.1-2.4.0
[0.30.0-2.4.0]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.29.0-2.3.0...0.30.0-2.4.0
[0.29.0-2.3.0]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.28.2-2.3.0...0.29.0-2.3.0
[0.28.2-2.3.0]: https://github.com/aurora-is-near/borealis-engine-lib/compare/0.28.1-1.39.1...0.28.2-2.3.0
[0.28.1-1.39.1]: https://github.com/aurora-is-near/borealis-engine-lib/compare/v0.10.0...0.28.1-1.39.1
[v0.10.0]: https://github.com/aurora-is-near/borealis-engine-lib/tree/v0.10.0

[@aleksuss]: https://github.com/aleksuss
[@birchmd]: https://github.com/birchmd
[@Casuso]: https://github.com/Casuso
[@mfornet]: https://github.com/mfornet
[@mrLSD]: https://github.com/mrLSD
[@raventid]: https://github.com/raventid
[@RomanHodulak]: https://github.com/RomanHodulak
[@spilin]: https://github.com/spilin
[@alexander-borodulya]: https://github.com/alexander-borodulya
