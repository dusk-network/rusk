# w3sper.js SDK

The W3sper.js SDK is a robust toolkit designed to streamline the development of applications that seamlessly interact with the Dusk Blockchain. This repository hosts the Web SDK, built with JavaScript, while a Native SDK, implemented in Rust, is also available in a separate repository. Both versions of the SDK offer comprehensive tools for managing blockchain interactions, but this version focuses on web-based implementations.

## Features

- **Address & Account Management**: Generate and manage profiles.
- **Balance & Transaction Handling**: Check balances, create signed transactions, and manage gas.
- **Event Subscription**: Subscribe to network events and query blockchain data.
- **Proof Management**: Generate and delegate cryptographic proofs.

## Modules Overview

| Module         | Native SDK | Web SDK | Description                              |
| -------------- | ---------- | ------- | ---------------------------------------- |
| Profile        | ✓          | ✓       | Manage accounts, addresses, and balances |
| Events         | ✓          | ✓       | Subscribe to and dispatch events         |
| Data (GraphQL) | ✓          | ✓       | Query blockchain data                    |
| Transaction    | ✓          | ✓       | Create and dispatch transactions         |
| Prover         | ✓          | ✗       | Generate cryptographic proofs            |

## Getting Started

To start using the W3sper SDK in your project, refer to the installation and usage guides for the [Native SDK](link-to-native-sdk-docs) and [Web SDK](link-to-web-sdk-docs).

## License

This project is licensed under the the [Mozilla Public License Version 2.0](./LICENSE).
