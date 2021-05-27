[![Build Status](https://travis-ci.com/dusk-network/rusk.svg?branch=master)](https://travis-ci.com/dusk-network/rusk)
[![codecov](https://codecov.io/gh/dusk-network/rusk/branch/master/graph/badge.svg)](https://codecov.io/gh/dusk-network/rusk)

# Rusk

The Dusk's Smart Contract Platform.

_Unstable_ : No guarantees can be made regarding the API stability, the project
is in development.

## Tests

To run tests:

```
source .env
make test
```

That will also compile all the genesis contracts and it's associated circuits.

## Use

To run the server:

```
make run
```

That will also compile all the genesis contracts.

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
