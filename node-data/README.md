<div align="center">

# `📊 Node Data`

> Core datatypes for the Dusk blockchain node — ledger structures, network messages, and wire-format encoding
</div>

## Overview

Node Data is the shared vocabulary between [`node`](../node/), [`consensus`](../consensus/), and [`rusk`](../rusk/). It defines all types that travel across the network or get persisted to the ledger, along with deterministic serialization logic for wire compatibility.

## Modules

| Module | Description |
|--------|-------------|
| `bls` | BLS public key wrappers with base58 and hex encoding |
| `ledger` | `Header`, `Block`, `SpentTransaction`, `Fault`, `Slash`, `Attestation` |
| `message` | Consensus message types with versioning and status tracking |
| `encoding` | `Serializable` trait for deterministic wire-format encoding |
| `events` | Block, transaction, and contract event types |

## Serialization

> **Warning**: Field ordering in serialized types is protocol-sensitive. Reordering fields or changing encoding can break network compatibility between nodes.

All wire-format types implement the `Serializable` trait, which provides `read` and `write` methods for deterministic binary encoding.

## Features

| Feature | Description |
|---------|-------------|
| `faker` | Generates test data via the `fake` crate |

## Related Crates

- [`dusk-core`](../core/) — foundation types (signatures, scalars)
- [`node`](../node/) — consumes ledger and message types for chain operations
- [`consensus`](../consensus/) — consumes message types for consensus protocol
- [`rusk`](../rusk/) — consumes event and ledger types for APIs
- [`rusk-wallet`](../rusk-wallet/) — consumes ledger types for transaction display
