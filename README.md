[![Build Status](https://travis-ci.com/dusk-network/rusk.svg?branch=master)](https://travis-ci.com/dusk-network/rusk)
[![codecov](https://codecov.io/gh/dusk-network/rusk/branch/master/graph/badge.svg)](https://codecov.io/gh/dusk-network/rusk)

# Rusk

The Dusk's Smart Contract Platform.

_Unstable_ : No guarantees can be made regarding the API stability, the project
is in development.

## Build and Tests

To run tests:

```
source .env
make test
```

That will also compile all the genesis contracts and it's associated circuits.

## Use

Prerequisites:

```
# Generate the keys used by the circuits
make keys

# Compile all the genesis contracts.
make wasm

# Copy example consensus.keys
mkdir -p ~/.dusk/rusk
cp examples/consensus.keys ~/.dusk/rusk/consensus.keys
```

Run a single-node cluster with example's data

```
# Generate genesis state
cargo r --release -p rusk-recovery --features state --bin rusk-recovery-state -- --init examples/genesis.toml -o /tmp/example.state

# Launch a local ephemeral node
DUSK_CONSENSUS_KEYS_PASS=password cargo r --release -p rusk -- -s /tmp/example.state
```


## Contracts compilation

To just compile all the genesis contracts without running the server:

```sh
make contracts
```

To generte a specific genesis contract:

```sh
# generate the wasm for `transfer` contract
make wasm for=transfer
```

See also `make help` for all the available commands
