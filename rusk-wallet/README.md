# Dusk Wallet CLI

A user-friendly, reliable command line interface to the Dusk wallet!

```
USAGE:
    rusk-wallet [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -d, --data-dir <DATA_DIR>
            Directory to store user data [default: `$HOME/.dusk`]

    -n, --wallet-name <NAME>
            Name for your wallet [default: `$(whoami)`]

    -f, --wallet-file <PATH>
            Path to a wallet file. Overrides `data-dir` and `wallet-name`,
            useful when loading a wallet that's not in the default directory

    -i, --ipc-method <IPC_METHOD>
            IPC method for communication with rusk [uds, tcp_ip]

    -r, --rusk-addr <RUSK_ADDR>
            Rusk address: socket path or fully quallified URL

    -p  --prover-addr <PROVER_ADDR>
            Prover service address

        --skip-recovery <SKIP_RECOVERY>
            Skip wallet recovery phrase (useful for headless wallet creation)

    -h, --help
            Print help information

    -V, --version
            Print version information

SUBCOMMANDS:
    create            Create a new wallet
    restore           Restore a lost wallet
    balance           Check your current balance
    address           Retrieve public spend key
    transfer          Send Dusk through the network
    stake             Start staking Dusk
    stake-info        Check your stake
    withdraw-stake    Withdraw a key's stake
    export            Export BLS provisioner key pair
    interactive       Run in interactive mode (default)
    help              Print this message or the help of the given subcommand(s)
```

## Good to know

Some commands can be run in standalone (offline) operation:
- `create`: Create a new wallet
- `restore`: Access (and restore) a lost wallet
- `address`: Retrieve public spend key
- `export`: Export BLS provisioner key pair

All other commands involve transactions, and thus require an active connection to [**Rusk**](https://github.com/dusk-network/rusk).

## Installation

[Install rust](https://www.rust-lang.org/tools/install) and then:

```
git clone https://github.com/dusk-network/rusk.git
cd rusk/rusk-wallet/
make install
```

## Configuring the CLI Wallet

You will need to connect to a running [**Rusk**](https://github.com/dusk-network/rusk) instance for full wallet capabilities.

Settings can be fed using a `config.toml` file. The CLI expects it to be either in your default data directory (`~/.dusk/config.toml`) or in the current working directory. The latter will be given priority if found first.

The user can override any particular configuration variable without having to manually edit the config file by explicitly passing the corresponding runtime argument(s) when running the CLI.

Here's an [example](config.toml) for reference.

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
