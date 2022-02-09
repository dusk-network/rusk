# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2022-02-08

### Changed 
- Default `gas_price` from 0 to 0.001 Dusk [#539]

## [0.2.0] - 2022-02-04

### Added
- Wallet file encoding version [#524]

### Changed
- Default to UDS transport [#520]

## [0.1.3] - 2022-02-01

### Added
- Offline mode [#499] [#507]
- Live validation to user interactive input
- Improved navigation through interactive menus
- "Pause" after command outputs for better readability

### Fixed
- Bad UX when creating an already existing wallet with default name

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

[#539]: https://github.com/dusk-network/rusk/issues/539
[#482]: https://github.com/dusk-network/rusk/issues/482
[#479]: https://github.com/dusk-network/rusk/issues/479
[#492]: https://github.com/dusk-network/rusk/issues/492
[#495]: https://github.com/dusk-network/rusk/issues/495
[#499]: https://github.com/dusk-network/rusk/issues/499
[#505]: https://github.com/dusk-network/rusk/issues/505
[#507]: https://github.com/dusk-network/rusk/issues/507
[#520]: https://github.com/dusk-network/rusk/issues/520
