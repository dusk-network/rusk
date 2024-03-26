[![Rusk CI](https://github.com/dusk-network/rusk/actions/workflows/rusk_ci.yml/badge.svg)](https://github.com/dusk-network/rusk/actions/workflows/rusk_ci.yml)
[![explorer CI](https://github.com/dusk-network/rusk/actions/workflows/explorer_ci.yml/badge.svg)](https://github.com/dusk-network/rusk/actions/workflows/explorer_ci.yml)
[![web-wallet CI](https://github.com/dusk-network/rusk/actions/workflows/webwallet_ci.yml/badge.svg)](https://github.com/dusk-network/rusk/actions/workflows/webwallet_ci.yml)
[![codecov](https://codecov.io/gh/dusk-network/rusk/branch/master/graph/badge.svg)](https://codecov.io/gh/dusk-network/rusk)

# Rusk

The official [Dusk](https://dusk.network/) protocol node client and smart contract platform.

_Unstable_ : No guarantees can be made regarding the API stability, the project
is in development.

## How to run a node

For more information on running a node, see our docs: 
- [Node Setup](https://docs.dusk.network/getting-started/node-setup/overview)
- [Node Requirements](https://docs.dusk.network/getting-started/node-setup/node-requirements)

## Prerequisites

- Rust 1.71 nightly or higher
- GCC 13 or higher
- Clang 16 or higher

### Rust Installation

Rusk makes use of the nightly toolchain, make sure it is installed. Furthermore, to build the WASM contracts, `wasm-pack` is required.

To install and set the nightly toolchain, and install `wasm-pack`, run:
```bash
rustup toolchain install nightly
rustup default nightly
cargo install wasm-pack
```

## Build and Tests

To build `rusk` from source, Rust, GCC and Clang are required. Once the dependencies are installed, you can simply run the following command to compile everything:

```bash
make
```

To run tests:

```bash
make test
```

That will also compile all the genesis contracts and its associated circuits.

## Use

Prerequisites:

```bash
# Generate the keys used by the circuits
make keys

# Compile all the genesis contracts
make wasm

# Copy example consensus.keys
mkdir -p ~/.dusk/rusk
cp examples/consensus.keys ~/.dusk/rusk/consensus.keys
```

Run a single full-node cluster with example state.

```bash
# Generate genesis state
cargo r --release -p rusk -- recovery-state --init examples/genesis.toml -o /tmp/example.state

# Launch a local ephemeral node
DUSK_CONSENSUS_KEYS_PASS=password cargo r --release -p rusk -- -s /tmp/example.state
```

### Prover Node

The node can be build as a prover only as follows:
```bash
cargo r --release --no-default-features --features prover -p rusk
```

This prover node will be accessible on `https://localhost:8080`. Apps like the [wallet-cli](https://github.com/dusk-network/wallet-cli) can be connected to it for quicker and more private local proving.

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
docker run -p 9000:9000/udp -p 8080:8080/tcp rusk
```

Port 9000 is used for Kadcast, port 8080 for the HTTP and GraphQL APIs.

## License

The Rusk software is licensed under the [Mozilla Public License Version 2.0](./LICENSE).
