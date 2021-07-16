# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [0.4.0] - 2021-07-16

## Added

- Add `transfer_contract` method for hosted [#328]
- Add `transfer_address` and `stake_address` methods for host [#328]

## Changed

- Update `dusk-abi` from `0.9.0-rc` to `0.9` [#327]
- Update `dusk-schnorr` from `0.7.0-rc` to `0.8.0-rc` [#327]
- Update `dusk-pki` from `0.7.0-rc` to `0.8.0-rc` [#327]
- Update `rusk-vm` from `0.6.0-rc` to `0.7.0-rc` [#327]

## [0.3.0] - 2021-05-14

### Added

- Add `payment_info` host function [#254]

### Changed

- Change `verify_proof` to accept verifier data [#247]
- Update `canonical` from `0.5` to `0.6`
- Update `canonical_derive` from `0.5` to `0.6`
- Update `dusk-poseidon` from `0.20` to `0.21.0-rc`
- Update `dusk-bls12_381` from `0.26` to `0.8`
- Update `dusk-abi` from `0.7` to `0.9-rc`
- Update `dusk-schnorr` from `0.6` to `0.7.0-rc`
- Update `dusk-pki` from `0.6` to `0.7.0-rc`
- Update `dusk-jubjub` from `0.8` to `0.10`
- Update `dusk-plonk` from `0.7` to `0.8`
- Update `rusk-vm` from `0.5` to `0.6.0-rc`
- Update `rusk-profile` from `0.3` to `0.4.0-rc`
- Replace `rand` version `0.7` with `rand_core` version `0.6`

### Remove

- Remove unused `tests/proof_test.bin`
- Remove unused `tests/vk_test.bin`

### Fix

- Fix the `repository` section in Cargo.toml

## [0.2.0] - 2021-03-12

### Added

- Add `verify_proof` host function [#227]
- Add `PublicInput` enum wrapper for input types
- Add `PublicParameters` as field of `RuskModule`
- Add Schnorr Signature verification host function

### Changed

- Change Build Status shield URL

### Removed

- Remove clippy warnings

## [0.1.0] - 2021-02-19

### Added

- Add ABI infrastracture
- Add Poseidon Hash host function
- Add test contract
- Add CHANGELOG.md
- Add LICENSE
- Add README.md

[#328]: https://github.com/dusk-network/rusk/issues/328
[#327]: https://github.com/dusk-network/rusk/issues/327
[#227]: https://github.com/dusk-network/rusk/issues/227
[#254]: https://github.com/dusk-network/rusk/issues/254
[unreleased]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.4.0...HEAD
[0.4.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.4.0
[0.3.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.3.0
[0.2.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.2.0
[0.1.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.1.0
