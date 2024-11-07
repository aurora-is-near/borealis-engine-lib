# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.29.0-2.3.0] 2024-11-07

### Changes

* chore: add test for check state init after processing batch transaction by [@aleksuss] in [#179]

### Fixes

* Fix handling mint transactions in aurora block processing by [@alexander_borodulya] in [#174]

[#174]: https://github.com/aurora-is-near/borealis-engine-lib/pull/174
[#179]: https://github.com/aurora-is-near/borealis-engine-lib/pull/179

## [0.28.2-2.3.0] 2024-10-31

### Changes
* chore: bump nearcore to 1.39.1 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/145
* chore: bump aurora-engine to 3.6.3 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/147
* chore(deps): bump rustls from 0.21.10 to 0.21.11 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/146
* chore: bump nearcore to 1.40.0-rc.1 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/148
* deps: bump nearcore to 1.40.0-rc.2 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/149
* chore: bump nearcore to 1.40.0 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/150
* chore: bump nearcore to 2.0.0-rc.1 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/153
* chore(deps): bump curve25519-dalek from 4.1.2 to 4.1.3 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/151
* deps: bump nearcore to 2.0.0-rc.2 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/154
* deps: bump nearcore to 2.0.0-rc.3 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/155
* deps: bump nearcore to 2.0.0-rc.4 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/156
* deps: bump nearcore to v2.0.0-rc.5 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/158
* Chore: Update toolchain and nearcore to 2.1.0-rc.1 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/159
* Chore: Update nearcore to v2.1.0-rc.2 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/160
* Chore: Update nearcore to v2.0.0 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/161
* Bump nearcore to 2.1.0-rc.3 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/162
* chore: Bump nearcore to 2.1.0 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/163
* chore: Bump nearcore to 2.1.1 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/164
* chore: Bump nearcore to 2.2.0-rc.1 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/165
* chore: bump nearcore to 2.2.0-rc.2 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/166
* chore: bump nearcore to 2.2.0 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/167
* chore: bump nearcore to 2.2.1 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/168
* chore: bump nearcore to 2.3.0-rc.1 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/170
* chore: bump nearcore to 2.3.0-rc.2 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/171
* chore: bump nearcore to 2.3.0-rc.3 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/172
* chore: bump nearcore to 2.3.0-rc.4 by @alexander-borodulya in https://github.com/aurora-is-near/borealis-engine-lib/pull/173
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
* Nearcore update to version 1.31.0 by @spilin in https://github.com/aurora-is-near/borealis-engine-lib/pull/44
* Fix: refiner can process receipts with no actions by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/45
* Fix: only prune tx tracker DB irregularly by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/46
* Fix: configuring the transaction tracker path is optional by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/48
* Fix: ignore whitespace in last block file by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/49
* Feat(NEARBlock): Implement `Clone`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/50
* Build(deps): Upgrade nearcore version `1.26.1` => `1.31.1`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/51
* Chore: update Engine and Near dependencies (support DelegateAction) by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/52
* Build(deps): Upgrade nearcore version `1.32.0-rc.1` => `1.32.0`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/53
* Build(deps): Update all dependencies; bump version. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/54
* Build(deps): Upgrade `nearcore` version `1.32.0` => `1.32.1`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/55
* Build(deps): Upgrade `nearcore` version `1.32.1` => `1.32.2`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/57
* Build(deps): Update all dependencies; bump version. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/58
* Build(deps): Upgrade `nearcore` version `1.32.2` => `1.33.0-rc.1`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/59
* Chore: upgrade near-lake-framework to version 0.7 by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/63
* chore(deps): bump h2 from 0.3.16 to 0.3.17 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/60
* Fix: if the execution outcome is a failure then it applies to the entire batch by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/64
* Chore: Update Cargo.toml to current release version by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/65
* Build(deps): Upgrade `nearcore` version `1.33.0-rc.1` => `1.33.0`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/66
* chore: update aurora-engine up to 2.9.1 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/67
* Release 0.20.0 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/68
* Build(deps): Upgrade `nearcore` version `1.33.0` => `1.34.0-rc.1`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/69
* Chore: upgrade to nearcore version 1.34.0 by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/70
* Feat: use generic modexp impl in borealis engine library by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/71
* Build(deps): Upgrade `nearcore` version `1.34.0` => `1.35.0-rc.1` and all other deps. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/75
* Add a unix socket server for handling RPC calls by @SamuelSarle in https://github.com/aurora-is-near/borealis-engine-lib/pull/73
* Feat: Validate `engine_path` before passing it to `init_storage`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/76
* Feat: Implement validation of Near account ID in config. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/77
* Refactor: Remove wrapper type `ValidatedAccountId`. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/78
* chore: update aurora-engine up to 2.10 by @vimpunk in https://github.com/aurora-is-near/borealis-engine-lib/pull/79
* Build(deps): Upgrade `nearcore` version `1.35.0-rc.1` => `1.35.0` and all other deps. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/80
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
* build(deps): update nearcore up to 1.36.0-rc.1 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/100
* feat: bump aurora-engine to 3.2.0 by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/102
* Feat: re-export `aurora-engine` and `nearcore` dependencies by @mrLSD in https://github.com/aurora-is-near/borealis-engine-lib/pull/103
* fix: pass ext-connector feature to dependencies by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/105
* Build(deps): Upgrade `nearcore` version `1.36.0-rc.1` => `1.36.0-rc.2` and all other deps. by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/106
* chore: bump aurora-engine to 3.3.1 and remove re-exports by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/107
* deps: bump nearcore to 1.36.0 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/108
* Fix(engine): Compute action_hash for each transaction by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/110
* feat: add label with version to prometheus metrics by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/109
* Feat(refiner): Include EVM logs for bridge transactions by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/111
* Build(deps): Bump `aurora-engine` version `7fff9ff` => `3.4.0` by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/113
* chore(deps): bump openssl from 0.10.57 to 0.10.60 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/112
* Build(deps): Bump `aurora-engine` version `3.4.0` => `3.5.0` by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/114
* Build(deps): Bump `nearcore` version `1.36.0` => `1.36.1` by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/115
* deps: bump nearcore to 1.36.2 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/117
* Fix(rpc): Estimate gas properly estimates contract deployments by @birchmd in https://github.com/aurora-is-near/borealis-engine-lib/pull/118
* chore: bump version up to 0.25.7 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/119
* Build(deps): Bump `nearcore` version `1.36.2` => `1.36.4` by @RomanHodulak in https://github.com/aurora-is-near/borealis-engine-lib/pull/120
* chore: bump rust to 1.74.0 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/121
* chore(deps): bump shlex from 1.2.0 to 1.3.0 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/123
* chore: bump nearcore to 1.37.0-rc.1 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/124
* chore(deps): bump nearcore 1.37.0-rc.1 => 1.37.0-rc.3 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/125
* chore(deps): bump aurora-engine to 3.6.0 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/126
* fix: use engine account id from storage in eth_call by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/127
* deps: bump nearcore to 1.37.0 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/130
* deps: bump nearcore to 1.37.1 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/131
* deps: bump nearcore to 1.38.0-rc.1 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/132
* chore: bump nearcore to 1.38.0-rc.2 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/133
* chore: bump nearcore to 1.38.0 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/136
* Build and push docker container on release by @spilin in https://github.com/aurora-is-near/borealis-engine-lib/pull/137
* chore: bump aurora-engine to 3.6.2 by @aleksuss in https://github.com/aurora-is-near/borealis-engine-lib/pull/138
* chore: bump nearcore to 1.38.1 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/139
* chore: add a dockerhub link to readme by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/140
* chore: bump nearcore to 1.38.2 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/141
* chore(deps): bump h2 from 0.3.24 to 0.3.26 by @dependabot in https://github.com/aurora-is-near/borealis-engine-lib/pull/143
* chore: bump nearcore to 1.39.0 by @raventid in https://github.com/aurora-is-near/borealis-engine-lib/pull/144

## [v0.10.0] 

[Unreleased]: https://github.com/aurora-is-near/borealis-engine-lib/0.29.0-2.3.0...main
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
[@alexander_borodulya]: https://github.com/alexander-borodulya
