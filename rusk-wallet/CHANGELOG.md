# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed
- `data_dir` can be properly overriden [#656]

## Added
- Notes cache [#650]
- Settings can be loaded from a config file [#637]
- Create config file if not exists [#647]
- Notify user when defaulting configuration [#655]
- Implementation for `State`'s `fetch_block_height` [#651]

## Changed
- Export consensus public key as binary
- Interactive mode allows for directory and wallet file overriding [#630]
- Client errors implemented, Rusk error messages displayed without metadata [#629]
- Transactions from wallets with no balance are halted immediately [#631]
- Rusk and prover connections decoupled [#659]


## [0.5.2] - 2022-03-01

## Added
- Optional configuration item to specify the prover URL [#612]
- Get Stake information subcommand [#619]

## [0.5.1] - 2022-02-26

## Added
- Display progress info about transaction preparation [#600]
- Display confirmation before sending a transaction [#602]

## Changed
- Use hex-encoded tx hashes on user-facing messages [#597]
- Open or display explorer URL on succesful transactions [#598]

## [0.5] - 2022-02-26

## Changed
- Update `canonical` across the entire Rusk stack [#606]

## [0.4.0] - 2022-02-17

## Changed
- Use the Dusk denomination from `rusk-abi` [#582]

## [0.3.1] - 2022-02-17

### Changed
- Default to current wallet directory for exported keys [#574]
- Add an additional plain text file with the base58-encoded public key [#574]

## [0.3.0] - 2022-02-17

### Removed
- Stake expiration [#566]

## [0.2.4] - 2022-02-15

### Added
- Allow for headless wallet creation [#569]

### Changed
- TX output in wallet instead of within client impl

## [0.2.3] - 2022-02-10

### Added
- Pretty print wallet-core errors [#554]

## [0.2.2] - 2022-02-10

### Changed
- Interactive mode prevents sending txs with insufficient balance [#547]

### Fixed
- Panic when UDS socket is not available

## [0.2.1] - 2022-02-09

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


[#582]: https://github.com/dusk-network/rusk/issues/582
[#482]: https://github.com/dusk-network/rusk/issues/482
[#479]: https://github.com/dusk-network/rusk/issues/479
[#492]: https://github.com/dusk-network/rusk/issues/492
[#495]: https://github.com/dusk-network/rusk/issues/495
[#499]: https://github.com/dusk-network/rusk/issues/499
[#505]: https://github.com/dusk-network/rusk/issues/505
[#507]: https://github.com/dusk-network/rusk/issues/507
[#520]: https://github.com/dusk-network/rusk/issues/520
[#539]: https://github.com/dusk-network/rusk/issues/539
[#547]: https://github.com/dusk-network/rusk/issues/547
[#554]: https://github.com/dusk-network/rusk/issues/554
[#566]: https://github.com/dusk-network/rusk/issues/566
[#569]: https://github.com/dusk-network/rusk/issues/569
[#574]: https://github.com/dusk-network/rusk/issues/574
[#597]: https://github.com/dusk-network/rusk/issues/597
[#598]: https://github.com/dusk-network/rusk/issues/598
[#600]: https://github.com/dusk-network/rusk/issues/600
[#602]: https://github.com/dusk-network/rusk/issues/602
[#606]: https://github.com/dusk-network/rusk/issues/606
[#612]: https://github.com/dusk-network/rusk/issues/612
[#619]: https://github.com/dusk-network/rusk/issues/619
[#629]: https://github.com/dusk-network/rusk/issues/629
[#630]: https://github.com/dusk-network/rusk/issues/630
[#631]: https://github.com/dusk-network/rusk/issues/631
[#637]: https://github.com/dusk-network/rusk/issues/637
[#647]: https://github.com/dusk-network/rusk/issues/647
[#650]: https://github.com/dusk-network/rusk/issues/650
[#651]: https://github.com/dusk-network/rusk/issues/651
[#655]: https://github.com/dusk-network/rusk/issues/655
[#656]: https://github.com/dusk-network/rusk/issues/656
[#659]: https://github.com/dusk-network/rusk/issues/659
