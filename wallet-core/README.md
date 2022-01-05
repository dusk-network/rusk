# Wallet Core

![status](https://github.com/dusk-network/wallet-core/workflows/Dusk%20CI/badge.svg)
[![codecov](https://codecov.io/gh/dusk-network/wallet-core/branch/main/graph/badge.svg?token=9W3J09AWZG)](https://codecov.io/gh/dusk-network/wallet-core)
[![documentation](https://img.shields.io/badge/docs-wallet-blue?logo=rust)](https://docs.rs/dusk-wallet-core/)

A library for generating and dealing with transactions.

## Build

To build and test the crate:

```shell
cargo b
cargo t
```

To build the WASM module:

```shell
cargo b --release --target wasm32-unknown-unknown
```
