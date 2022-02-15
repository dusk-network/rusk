# Dusk Wallet CLI

A user-friendly, reliable command line interface to the Dusk wallet!

```
USAGE:
    rusk-wallet [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -d, --data-dir <DATA_DIR>          Directory to store user data [default: /Users/$(whoami)/.dusk]
    -n, --wallet-name <NAME>           Name for your wallet [default: $(whoami).dat]
    -f, --wallet-file <PATH>           Path to a wallet file. Overrides `data-dir` and `wallet-
                                       name`, useful when loading a wallet that's not in the default
                                       directory
    -a, --rusk-addr <RUSK_ADDR>        Rusk address [default: 127.0.0.1]
    -p, --rusk-port <RUSK_PORT>        Rusk port [default: 8585]
    -i, --ipc-method <IPC_METHOD>      IPC method for communication with rusk [uds, tcp_ip]
                                       [default: uds]
    -s, --socket-path <SOCKET_PATH>    Path for setting up the unix domain socket [default: /tmp/
                                       rusk_listener]
        --skip-recovery                Skip wallet recovery phrase (useful for headless wallet
                                       creation)
    -h, --help                         Print help information
    -V, --version                      Print version information

SUBCOMMANDS:
    create            Create a new wallet
    restore           Restore a lost wallet
    balance           Check your current balance
    address           Retrieve public spend key
    transfer          Send Dusk through the network
    stake             Start staking Dusk
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
make build
```

## Running the CLI

As previously mentioned, you should ideally have a [**Rusk**](https://github.com/dusk-network/rusk) instance running for full wallet capabilities.

### Interactive mode

By default, the CLI runs in interactive mode when no arguments are provided.

```
cargo r --release
```

### Headless mode

Wallet can be run in headless mode by providing all the options required for a given subcommand. 

To explore available options and commands, use the `help` command:
```
cargo r --release -- help
```

To further explore any specific command you can use `--help` on the command itself, for example:
```
cargo r --release -- stake --help
```

By default, you will always be prompted to enter the wallet password. To prevent this behavior, you can provide the password using the `RUSK_WALLET_PWD` environment variable. This is useful in CI or any other headless environment.

Please note that `RUSK_WALLET_PWD` is effectively used for:
- Wallet decryption (in all commands that use a wallet)
- Wallet encryption (in `create`)
- BLS key encryption (in `export`)
