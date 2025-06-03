<div align="center">

# `🌒 Rusk`

> Entrypoint for the blockchain node
</div>

## Configure example's data

When running `prepare-dev` in the root repository, the Genesis state according to your local <a href="https://github.com/dusk-network/rusk/blob/master/examples/genesis.toml" target="_blank">`examples/genesis.toml`</a> will be used. Refer to <a href="https://github.com/dusk-network/rusk/blob/master/rusk-recovery/config/example.toml" target="_blank">`examples.toml`</a> for configuration options you can set, such as stakes and balances on network initialization.

Note that the `password` used when running rusk is connected to the example consensus keys, which are also defined in the <a href="https://github.com/dusk-network/rusk/blob/master/examples/genesis.toml" target="_blank">`examples/genesis.toml`</a>.

## Join a cluster

It is possible to connect to other clusters by defining a set of bootstrapping nodes to which to connect to on initialization, by defining them in the <a href="https://github.com/dusk-network/rusk/blob/master/rusk/default.config.toml#L13" target="_blank">`rusk/default.config.toml`</a> , or by passing the `--bootstrap` argument in the node launch command.
