[![Repository](https://img.shields.io/badge/github-dusk--wallet-purple?logo=github)](https://github.com/dusk-network/rusk/tree/master/rusk-wallet)
![Build Status](https://github.com/dusk-network/rusk/workflows/rusk-wallet%20CI/badge.svg)
[![Documentation](https://img.shields.io/badge/docs-dusk--wallet-orange?logo=rust)](https://docs.rs/dusk-wallet/)

# Dusk Wallet

Library providing functionalities to create wallets compatible with the Dusk network.

This library is used to implement the official [Dusk CLI wallet](https://github.com/dusk-network/rusk/blob/master/rusk-wallet/src/bin/README.md).

## Overview

Rusk Wallet is both a Rust library and a CLI application for interacting with the Dusk network. It supports multi-account management, Phoenix (shielded) and Moonlight (public) transactions, staking operations, and contract interaction.

## Key Features

| Feature | Description |
|---------|-------------|
| Interactive CLI | Terminal UI for wallet operations |
| Multi-account | Profile and account management |
| Encrypted storage | AES-GCM encrypted wallet files backed by RocksDB |
| Node communication | GraphQL client for querying the Rusk node |
| Dual transaction models | Phoenix (shielded/UTXO) and Moonlight (public/account) support |

## Related Crates

- [`wallet-core`](../wallet-core/) — low-level wallet primitives (key derivation, note handling, transaction construction)
- [`node-data`](../node-data/) — ledger types for transaction display
- [`dusk-core`](../core/) — cryptographic signatures and transaction types
