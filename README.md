# Aurora Standalone

[![CI](https://github.com/aurora-is-near/borealis-engine-lib/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/aurora-is-near/borealis-engine-lib/actions/workflows/rust.yml)

A collection of Rust crates useful for local ("standalone") deployments of Aurora infrastructure (tooling indexing data about an on-chain Aurora Engine deployment).

## Refiner Application

Refiner allows users to download all NEAR Blocks and get all information relevant to Aurora. NEAR Blocks data can be consumed from two different sources:
- [NEAR data lake](https://github.com/near/near-lake-framework-rs): Downloading NEAR Blocks from AWS.
- [NEARCore](https://github.com/near/nearcore): Running an archival nearcore instance.

To run the refiner make sure you have [rust installed](https://www.rust-lang.org/tools/install). Clone this repository, and run the relevant command within the repository path.

The refiner will save all refined blocks in json files on the specified output path. By default it is `output/refiner/*/block.json`. Blocks will be written sequentially. If the refiner is restarted, it will start from the last block processed.

### NEAR Data Lake

1. Setup AWS locally. Check [this tutorial](https://youtu.be/GsF7I93K-EQ?t=277) to see how to do it.
2. [Optional] Check `default_config.json` parameters.
3. Run the binary:

```
cargo run --release -- -c default_config.json run
```

### NEARCore

1. Check `nearcore_config.json` parameters. Set `input_mode.Nearcore.path` to the path of the `nearcore` data. (Where `config.json` is located).
2. Run the binary:

```
cargo run --release -- -c nearcore_config.json run
```

With this approach a nearcore instance will be launched, and will be syncing with the network. It is ok to download any valid snapshot and start from there. Starting from scratch can take several days(?).
