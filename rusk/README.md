<div align="center">

# `🌒 Rusk`

> Entrypoint for the blockchain node
</div>

## Run rusk locally:

### Run a single-node cluster with example's data

Create a new directory and copy the example consensus keys to it. In a production environment, you would put your own consensus keys here.
```bash
mkdir -p ~/.dusk/rusk
cp examples/consensus.keys ~/.dusk/rusk/consensus.keys
```

Create the Genesis state according to your local <a href="https://github.com/dusk-network/rusk/blob/master/examples/genesis.toml" target="_blank">`examples/genesis.toml`</a>. Refer to <a href="https://github.com/dusk-network/rusk/blob/master/rusk-recovery/config/example.toml" target="_blank">`examples.toml`</a> for configuration options you can set, such as stakes and balances on network initialization.

### Run ephemeral node

```bash
# Generate genesis state
cargo r --release -p rusk -- recovery-state --init examples/genesis.toml -o /tmp/example.state

# Launch a local ephemeral node
DUSK_CONSENSUS_KEYS_PASS=password cargo r --release -p rusk -- -s /tmp/example.state -c rusk/default.config.toml
```

### Run persistent node

Delete any leftover in state/chain
```bash
# Delete old state
rm -rf ~/.dusk/rusk/state
# Delete old chain
rm -rf ~/.dusk/rusk/chain.db
```

```bash
# Generate genesis state
cargo r --release -p rusk -- recovery-state --init examples/genesis.toml

# Launch a local node
DUSK_CONSENSUS_KEYS_PASS=password cargo r --release -p rusk -- -c rusk/default.config.toml
```

Note that the `password` used here is connected to the example consensus keys, which are also defined in the <a href="https://github.com/dusk-network/rusk/blob/master/examples/genesis.toml" target="_blank">`examples/genesis.toml`</a>.

## Join a cluster

It is possible to connect to other clusters by defining a set of bootstrapping nodes to which to connect to on initialization, by defining them in the <a href="https://github.com/dusk-network/rusk/blob/master/rusk/default.config.toml#L13" target="_blank">`rusk/default.config.toml`</a> , or by passing the `--bootstrap` argument in the node launch command.

### Genesis.toml

The genesis toml in ../examples is the one used when running a single node cluster. You can adjust it to your own settings.
