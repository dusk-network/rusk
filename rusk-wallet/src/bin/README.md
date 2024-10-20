# Rusk Wallet

A feature-rich CLI wallet for interacting with Dusk.

```
USAGE:
    rusk-wallet [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -p, --profile <PROFILE>        Directory to store user data [default: `$HOME/.dusk/rusk-wallet`]
    -n, --network <NETWORK>        Network to connect to
        --password <PASSWORD>      Set the password for wallet's creation [env:
                                   RUSK_WALLET_PWD=password]
        --state <STATE>            The state server fully qualified URL
        --prover <PROVER>          The prover server fully qualified URL
        --log-level <LOG_LEVEL>    Output log level [default: info] [possible values: trace, debug,
                                   info, warn, error]
        --log-type <LOG_TYPE>      Logging output type [default: coloured] [possible values: json,
                                   plain, coloured]
    -h, --help                     Print help information
    -V, --version                  Print version information

SUBCOMMANDS:
    create                   Create a new wallet
    restore                  Restore a lost wallet
    balance                  Check your current balance
    profiles                 List your existing profiles and generate new ones
    history                  Show address transaction history
    transfer                 Send DUSK through the network
    phoenix-to-moonlight     Convert shielded DUSK to public Dusk (the conversion will happen
                                 between the shielded and public addresses of the same profile)
    moonlight-to-phoenix     Convert public DUSK to shielded Dusk (the conversion will happen
                                 between the shielded and public addresses of the same profile)
    stake                    Stake DUSK
    unstake                  Unstake DUSK
    withdraw                 Withdraw accumulated rewards for a stake key
    contract-deploy          Deploy a contract
    stake-info               Check your stake information
    contract-call            Call a contract
    calculate-contract-id    Command to calculate the contract id given the contract code and
                                 deploy nonce
    export                   Export BLS provisioner key-pair
    settings                 Show current settings
    help                     Print this message or the help of the given subcommand(s)
```

## Good to know

Some commands can be run in standalone (offline) operation:

- `create`: Create a new wallet
- `restore`: Access (and restore) a lost wallet
- `addresses`: Retrieve your addresses
- `export`: Export BLS provisioner key pair

All other commands involve transactions, and thus require an active connection to [**Rusk**](https://github.com/dusk-network/rusk).

## Installation

[Install rust](https://www.rust-lang.org/tools/install) and then:

```
git clone git@github.com:dusk-network/rusk.git
cd rusk/rusk-wallet
make install
```

## Configuring the CLI Wallet

You will need to connect to a running [**Rusk**](https://github.com/dusk-network/rusk) instance for full wallet capabilities.

The default settings can be seen [here](https://github.com/dusk-network/rusk/blob/master/rusk-wallet/default.config.toml).

It's possible to override those settings by create a `config.toml` file with the same structure, in one of the following
directory:

- The profile folder (provided via the `--profile` argument, defaults to `$HOME/.dusk/rusk-wallet/`)
- The global configuration folder (`$HOME/.config/rusk-wallet/`)

Having the `config.toml` in the global configuration folder is useful in case of multiple wallets (each one with its own profile folder) that shares the same settings.

If a `config.toml` exists in both locations, the one found in the profile folder will be used.

The CLI arguments takes precedence and overrides any configuration present in the configuration file.

**Note:** When using Windows, connection will default to TCP/IP even if UDS is explicitly specified.

## Running the CLI Wallet

### Interactive mode

By default, the CLI runs in interactive mode when no arguments are provided.

```
rusk-wallet
```

### Headless mode

Wallet can be run in headless mode by providing all the options required for a given subcommand. It is usually convenient to have a config file with the wallet settings, and then call the wallet with the desired subcommand and its options.

To explore available options and commands, use the `help` command:

```
rusk-wallet help
```

To further explore any specific command you can use `--help` on the command itself. For example, the following command will print all the information about the `stake` subcommand:

```
rusk-wallet stake --help
```

By default, you will always be prompted to enter the wallet password. To prevent this behavior, you can provide the password using the `RUSK_WALLET_PWD` environment variable. This is useful in CI or any other headless environment.

Please note that `RUSK_WALLET_PWD` is effectively used for:

- Wallet decryption (in all commands that use a wallet)
- Wallet encryption (in `create`)
- BLS key encryption (in `export`)
