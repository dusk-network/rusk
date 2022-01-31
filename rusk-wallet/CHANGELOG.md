# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [0.1.2] - 2022-01-31

### Added
- Enable headless mode [#495]
- Introduce interactive mode by default [#492]
- Add Export command for BLS PubKeys [#505]

## [0.1.1] - 2022-01-27

### Added
- Wallet file encryption using AES [#482]

### Changed
- Common `Error` struct for this crate [#479]
- Password hashing using blake3

### Removed
- Recovery password

## [0.1.0] - 2022-01-25

### Added
- `rusk-wallet` crate to workspace
- Argument and command parsing, with help output
- Interactive prompts for authentication
- BIP39 mnemonic support for recovery phrase
- Implementation of `Store` trait from `wallet-core`
- Implementation of `State` and `Prover` traits from `wallet-core`


[#482]: https://github.com/dusk-network/rusk/issues/482
[#479]: https://github.com/dusk-network/rusk/issues/479
[#492]: https://github.com/dusk-network/rusk/issues/492
[#495]: https://github.com/dusk-network/rusk/issues/495
[#505]: https://github.com/dusk-network/rusk/issues/505