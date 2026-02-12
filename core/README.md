<div align="center">

# `🧬 Dusk-core`

> Core types used within Dusk, for writing smart contracts and interacting with Dusk
</div>

## Overview

Dusk-core is the foundation crate for the entire Rusk workspace. It provides the cryptographic primitives, transaction types, and contract ABI that nearly every other crate depends on. It is `no_std` compatible, making it suitable for use inside WASM smart contracts.

## What It Provides

| Area | Description |
|------|-------------|
| Signatures | BLS signatures (bls12-381) and Schnorr signatures (JubJub) |
| Zero-knowledge | PLONK and Groth16 circuit types and proof structures |
| Transactions | Types for both Phoenix (shielded/UTXO) and Moonlight (public/account) models |
| Contract ABI | Host functions and interfaces for smart contract development |
| Serialization | `rkyv` and `dusk-bytes` based encoding for all core types |

## abi / abi-dlmalloc feature

When importing core with `abi-dlmalloc`, a smart contract developer on Dusk is able to use the abi host functions provided through it.

The current available host functions can be seen in the host_queries module in [abi.rs](./src/abi.rs)

## Related Crates

Dusk-core is foundational — it is depended on by nearly every other crate in the workspace, including [`node-data`](../node-data/), [`consensus`](../consensus/), [`node`](../node/), [`vm`](../vm/), [`rusk`](../rusk/), [`wallet-core`](../wallet-core/), [`rusk-prover`](../rusk-prover/), and the [contracts](https://github.com/dusk-network/contracts).
