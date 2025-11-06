# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Added support for generic TransactionData into FFI [#3750]

### Changed

- Changed eip2333 public functions `derive_master_sk`, `derive_child_sk`, `derive_bls_sk` [#3681]

## [1.3.0] - 2025-04-17

### Added

- Added support for EIP2333 BLS key derivation [#3476]

### Changed

- Changed `dusk-core` version to `1.3.0`

## [1.1.0] - 2025-02-14

### Changed

- Changed phoenix function to allow data to be passed to transaction [#3438] 

## [1.0.1] - 2025-01-23

### Changed

- Change dependency declaration to not require strict equal [#3405]

## [1.0.0] - 2025-01-20

- First `dusk-wallet-core` release

<!-- Issues -->
[#3750]: https://github.com/dusk-network/rusk/issues/3750
[#3681]: https://github.com/dusk-network/rusk/issues/3681
[#3476]: https://github.com/dusk-network/rusk/issues/3476
[#3438]: https://github.com/dusk-network/rusk/issues/3438
[#3405]: https://github.com/dusk-network/rusk/issues/3405

[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-wallet-core-1.3.0...HEAD
[1.3.0]: https://github.com/dusk-network/rusk/compare/dusk-wallet-core-1.1.0...dusk-wallet-core-1.3.0
[1.1.0]: https://github.com/dusk-network/rusk/compare/dusk-wallet-core-1.0.1...dusk-wallet-core-1.1.0
[1.0.1]: https://github.com/dusk-network/rusk/compare/wallet-core-1.0.0...dusk-wallet-core-1.0.1
[1.0.0]: https://github.com/dusk-network/rusk/tree/wallet-core-1.0.0
