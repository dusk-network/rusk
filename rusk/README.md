<div align="center">

# `🌒 Rusk`

> Entrypoint for the blockchain node
</div>

## Overview

Rusk is the main binary and orchestration layer for the Dusk blockchain. It wires together the chain node, consensus engine, contract VM, and exposes HTTP/GraphQL APIs for wallets and applications to interact with the network.

## Node Modes

| Mode | Flag | Description |
|------|------|-------------|
| Provisioner | *(default)* | Full consensus participation — proposes and validates blocks |
| Archive | `--features archive` | Historical data indexing via SQLite for explorers and analytics |
| Prover | `--features prover` | Local ZK proving service for Phoenix transactions |

## Key Modules

| Module | Description |
|--------|-------------|
| `http` | GraphQL server for blockchain queries and transaction submission |
| `node` | Integration point for chain + consensus + networking |
| `verifier` | Proof verification (PLONK, Groth16) for incoming transactions |

## Related Crates

- [`dusk-vm`](../vm/) — contract execution engine
- [`dusk-core`](../core/) — transaction and cryptographic types
- [`node`](../node/) — chain node (networking, storage, mempool)
- [`dusk-consensus`](../consensus/) — block ordering and finality
- [`rusk-profile`](../rusk-profile/) — circuit artifact management
- [`rusk-prover`](../rusk-prover/) — local ZK prover (optional)
- [`rusk-recovery`](../rusk-recovery/) — state and key bootstrapping
- [`node-data`](../node-data/) — ledger and message types

## Configure example's data

When running `prepare-dev` in the root repository, the Genesis state according to your local <a href="https://github.com/dusk-network/rusk/blob/master/examples/genesis.toml" target="_blank">`examples/genesis.toml`</a> will be used. Refer to <a href="https://github.com/dusk-network/rusk/blob/master/rusk-recovery/config/example.toml" target="_blank">`examples.toml`</a> for configuration options you can set, such as stakes and balances on network initialization.

Note that the `password` used when running rusk is connected to the example consensus keys, which are also defined in the <a href="https://github.com/dusk-network/rusk/blob/master/examples/genesis.toml" target="_blank">`examples/genesis.toml`</a>.

## Join a cluster

It is possible to connect to other clusters by defining a set of bootstrapping nodes to which to connect to on initialization, by defining them in the <a href="https://github.com/dusk-network/rusk/blob/master/rusk/default.config.toml#L13" target="_blank">`rusk/default.config.toml`</a> , or by passing the `--bootstrap` argument in the node launch command.
