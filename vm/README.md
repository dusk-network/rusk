![Build Status](https://github.com/dusk-network/rusk/actions/workflows/rusk_ci.yml/badge.svg)
[![Repository](https://img.shields.io/badge/github-dusk--vm-blueviolet?logo=github)](https://github.com/dusk-network/rusk)
[![Documentation](https://img.shields.io/badge/docs-dusk--vm-blue?logo=rust)](https://docs.rs/dusk-vm/)

# Dusk VM

The Dusk VM is a virtual machine designed for **Dusk**, enabling secure and efficient execution of smart contracts, state transitions, and cryptographic operations tailored for zero-knowledge-based applications.

It serves as the execution engine of the Dusk Blockchain, leveraging advanced cryptographic primitives and frameworks to support privacy-preserving, compliant and scalable decentralized applications.

## Features

- **State Management**: Manage blockchain state using sessions for isolated transaction execution and finalization.
- **Cryptographic Support**: Offers built-in support for hashing (Poseidon), signature verification (BLS, Schnorr), and proof validation (PLONK, Groth16).
- **Virtual Machine for zk-SNARK Applications**: Optimized for privacy-preserving computations.

## Installation

Add `dusk-vm` to your `Cargo.toml`:

```toml
[dependencies]
dusk-vm = "0.x"  # Replace with the latest version
```

## Documentation

For detailed usage and API examples, refer to the [crate documentation on docs.rs](https://docs.rs/dusk-vm/).

## License

License: MPL-2.0
