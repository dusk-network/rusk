# w3sper SDK

`w3sper` is a JavaScript library designed to enhance secure and private
communication within the Dusk Network ecosystem. With `w3sper`, developers can
integrate privacy-focused blockchain features seamlessly into web applications,
harnessing the power of the Dusk Network’s privacy-centric technology.

## Key Features

- **Address & Account Management**: Easily create and manage profiles for both
  public accounts and shielded addresses, supporting secure and private identity
  handling.
- **Balance & Transaction Management**: Access account balances, transfer tokens
  between shielded addresses and public accounts, create and sign transactions,
  and manage gas efficiently.
- **Offline Transaction Creation**: Generate signed transactions offline for
  public accounts, enabling secure transaction handling without an internet
  connection.
- **Event Subscription**: Subscribe to network and contract events in real time,
  allowing seamless access to blockchain updates.
- **Contract Interactions**: Query contract states and create custom
  transactions for contract interactions, making decentralized application
  development more flexible.
- **Proof Management**: Generate and delegate cryptographic proofs for enhanced
  privacy and security.

## Installation

To install the `w3sper` SDK, run the following command:

```sh
deno add jsr:@dusk/w3sper
```

Installation

## Development

To set up `w3sper` locally, follow these steps:

### Prerequisites

Make sure you have the following dependencies installed:

- [Deno](https://deno.com/)
- [Rust](https://www.rust-lang.org/) and Cargo for
  [wallet-core](https://github.com/dusk-network/rusk/tree/master/wallet-core)
  compilation

### Compilation Steps

**Compile wallet-core WASM**

The `wasm` task is used to compile the WASM module from `wallet-core`. Run the
following command:

```bash
deno task wasm
```

**Prepare the local node state**

The `state` task initializes the required state for running a local Dusk node.
Run:

```bash
deno task state
```

**Start the local node**

Use the `rusk` task to start the local Dusk node:

```bash
deno task rusk
```

**Run tests**

Execute the tests to verify everything is working as expected:

```bash
deno task test
```

## License

This project is licensed under the the
[Mozilla Public License Version 2.0](./LICENSE).
