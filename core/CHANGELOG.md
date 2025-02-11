# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Change `dusk_core::transfer::moonlight::Transaction::data` fn visibility to public

### Added

- Add `METADATA::PUBLIC_SENDER` [#3341]
- Add `abi::public_sender` host fn [#3341]
- Add serde implementations for `StakeEvent`, `SlashEvent`, `Reward`, `WithdrawEvent`, `ConvertEvent`, `DepositEvent`,
`ContractToContractEvent`, `ContractToAccountEvent`, `PhoenixTransactionEvent` and `MoonlightTransactionEvent`

## [1.0.0] - 2025-01-23

### Changed

- Change dependency declaration to not require strict equal [#3405]

## [0.1.0] - 2025-01-14


### Added

- Add types, type-alias, functionality, re-exports and modules to interact with dusk-network

<!-- Issues -->
[#3405]: https://github.com/dusk-network/rusk/issues/3405
[#3341]: https://github.com/dusk-network/rusk/issues/3341

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-core-1.0.0...HEAD
[1.0.0]: https://github.com/dusk-network/rusk/compare/dusk-core-0.1.0...dusk-core-1.0.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/dusk-core-0.1.0
