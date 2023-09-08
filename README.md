[![Build Status](https://travis-ci.com/dusk-network/rusk.svg?branch=master)](https://travis-ci.com/dusk-network/rusk)
[![codecov](https://codecov.io/gh/dusk-network/rusk/branch/master/graph/badge.svg)](https://codecov.io/gh/dusk-network/rusk)

# Rusk

The official [Dusk](https://dusk.network/) protocol node client and smart contract platform.

_Unstable_ : No guarantees can be made regarding the API stability, the project
is in development.

## Prerequisites

- Rust 1.71 nightly or higher
- GCC 13 or higher
- Clang 16 or higher

## Specification Requirements

### Minimum Specifications

| CPU | RAM | Storage | Network Connection |
| :--- | :--- | :--- | :--- |
| 2 cores; 2 GHz | 1 GB | 60 GB | 1 Mbps |

### Recommended Specifications

| CPU | RAM | Storage | Network Connection |
| :--- | :--- | :--- | :--- |
| 4 cores; 2 GHz | 4 GB | 250 GB | 10 Mbps |

## Build and Tests

To build `rusk` from source, Rust, GCC and Clang are required. Once the dependencies are installed, you can simply run the following command to compile everything:

```bash
source .env
make
```

To run tests:

```bash
source .env
make test
```

That will also compile all the genesis contracts and its associated circuits.

## Use

Prerequisites:

```bash
# Required for the generation of the keys
source .env

# Generate the keys used by the circuits
make keys

# Compile all the genesis contracts
make wasm

# Copy example consensus.keys
mkdir -p ~/.dusk/rusk
cp examples/consensus.keys ~/.dusk/rusk/consensus.keys
```

Run a single-node cluster with example's data

```bash
# Generate genesis state
cargo r --release -p rusk-recovery --features state --bin rusk-recovery-state -- --init examples/genesis.toml -o /tmp/example.state

# Launch a local ephemeral node
DUSK_CONSENSUS_KEYS_PASS=password cargo r --release -p rusk -- -s /tmp/example.state
```

## Contracts compilation

To just compile all the genesis contracts without running the server:

```bash
make contracts
```

To generate a specific genesis contract:

```bash
# generate the wasm for `transfer` contract
make wasm for=transfer
```

See also `make help` for all the available commands

## Docker support

It's also possible to run a local ephemeral node with Docker.

To build the Docker image:

```bash
docker build -t rusk .
```

To run Rusk inside a Docker container:

```bash
docker run -p 9000:9000/udp rusk
```

## How to run a node

For more information on running a node, see our wiki: 
- [Setting up a node](https://wiki.dusk.network/en/setting-up-node)
- [Setting up a node with Docker](https://wiki.dusk.network/en/setting-up-a-node-docker)

## License

The Rusk software is licensed under the [Mozilla Public License Version 2.0](./LICENSE).
