# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased


## [0.1.0] - 2022-01-25

### Added
- `rusk-wallet` crate to workspace
- Argument and command parsing, with help output
- Interactive prompts for authentication
- BIP39 mnemonic support for recovery phrase
- Implementation of `Store` trait from `wallet-core`
- Implementation of `State` and `Prover` traits from `wallet-core`
- gRPC clients linked to Rusk services
- `CliError` type for all crate errors
- Support for wallet file encryption

### Changed

### Removed