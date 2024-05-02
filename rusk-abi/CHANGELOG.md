# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Memoize the `verify_proof` function [#1228]

### Changed

- Change dependencies declarations enforce bytecheck [#1371]
- Update `piecrust` from `0.15` to `0.16`
- Update `piecrust-uplink` from `0.10` to `0.11`
- Update dusk dependencies [#1609]

## [0.11.0] - 2023-10-12

### Changed

- Update `dusk-bls12_381` from `0.11` to `0.12`
- Update `dusk-bls12_381-sign` from `0.4` to `0.15`
- Update `dusk-jubjub` from `0.12` to `0.13`
- Update `dusk-poseidon` from `0.30` to `0.31`
- Update `dusk-pki` from `0.12` to `0.13`
- Update `dusk-plonk` from `0.14` to `0.16`

### Added

- Add `ff` dev-dependency `0.13`

### Changed

- Update `piecrust` from `0.8.0-rc` to `0.10`
- Update `piecrust` from `0.7` to `0.8.0-rc`
- Update `piecrust` from `0.6` to `0.7`
- Update `piecrust-uplink` from `0.6` to `0.7`

## [0.10.0-piecrust.0.6] - 2023-07-06

### Changed

- Update `plonk` from `0.13` to `0.14` [#929]

## [0.9.0-piecrust.0.6] - 2023-07-04

### Changed

- Update `piecrust` to `0.6` [#945]
- Use `dusk-merkle` instead of `microkelvin` [#937]
- Update `dusk-poseidon` from `0.28` to `0.29.1-rc.0`

## [0.8.0-piecrust.0.5] - 2023-06-13

### Changed

- Update `piecrust` to `0.1.0`

### Fix

- Fix `rkyv` and `wasmer` deps semver [#874]

## [0.8.0-alpha] - 2023-03-27

### Changed

- Change `rusk-abi` to use `piecrust` [#760]
- Change `rusk-abi` to use `piecrust-uplink` [#760]

## [0.7.0] - 2022-02-25

### Changed

- Update canonical from `0.6` to `0.7`
- Update canonical_derive from `0.6` to `0.7`
- Update dusk-poseidon from `0.23.0-rc` to `0.25.0-rc`
- Update dusk-bls12_381 from `0.8` fro `0.9`
- Update dusk-bls12_381-sign `0.1.0-rc` to `0.3.0-rc`
- Update dusk-abi from `0.10` to `0.11`
- Update dusk-schnorr from `0.9.0-rc` to `0.10.0-rc`
- Update dusk-pki from `0.9.0-rc` to `0.10.0-rc`
- Update dusk-jubjub from `0.10` to `0.11`
- Update rusk-vm from `0.10.0-rc` to `0.12.0-rc`
- Update dusk-plonk from `0.9` to `0.10`

## [0.6.3] - 2022-02-19

### Changed

- Update `rusk-vm` from `0.8.0-rc` to `0.10.0-rc`

## [0.6.2] - 2022-01-28

### Added

- Add BLS signature verification to `rusk-abi` [#475]

## [0.6.0] - 2022-01-24

### Changed

- Update dependencies to most recent [#430] [#433]
- Update `rusk-vm` from `0.7.0-rc` to `0.8.0-rc`

## [0.5.0] - 2022-01-11

### Changed

- Change hashing of transaction to inside the contract [#402]
- Update to `dusk-plonk` to `0.9` [#392]

## [0.4.1] - 2021-07-28

### Added

- Add `stake_contract` method for host and hosted [#338]
- Add `transfer_contract` method for host [#338]
- Add `gen_contract_id` method for host [#337]

### Remove

- Remove `transfer_address` and `stake_address` methods for host [#338]

## [0.4.0] - 2021-07-16

### Added

- Add `transfer_contract` method for hosted [#328]
- Add `transfer_address` and `stake_address` methods for host [#328]

### Changed

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

[#1609]: https://github.com/dusk-network/rusk/issues/1609
[#1371]: https://github.com/dusk-network/rusk/issues/1371
[#1228]: https://github.com/dusk-network/rusk/issues/1228
[#945]: https://github.com/dusk-network/rusk/issues/945
[#937]: https://github.com/dusk-network/rusk/issues/937
[#874]: https://github.com/dusk-network/rusk/issues/874
[#760]: https://github.com/dusk-network/rusk/issues/760
[#475]: https://github.com/dusk-network/rusk/issues/475
[#433]: https://github.com/dusk-network/rusk/issues/433
[#430]: https://github.com/dusk-network/rusk/issues/430
[#402]: https://github.com/dusk-network/rusk/issues/402
[#392]: https://github.com/dusk-network/rusk/issues/392
[#338]: https://github.com/dusk-network/rusk/issues/338
[#337]: https://github.com/dusk-network/rusk/issues/337
[#328]: https://github.com/dusk-network/rusk/issues/328
[#327]: https://github.com/dusk-network/rusk/issues/327
[#227]: https://github.com/dusk-network/rusk/issues/227
[#254]: https://github.com/dusk-network/rusk/issues/254

[Unreleased]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.11.0...HEAD
[0.11.0]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.10.0-piecrust.0.6...rusk-abi-0.11.0
[0.10.0-piecrust.0.6]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.9.0-piecrust.0.6...rusk-abi-0.10.0-piecrust.0.6
[0.9.0-piecrust.0.6]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.8.0-piecrust.0.5...rusk-abi-0.9.0-piecrust.0.6
[0.8.0-piecrust.0.5]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.8.0-alpha...rusk-abi-0.8.0-piecrust.0.5
[0.8.0-alpha]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.7.0...rusk-abi-0.8.0-alpha
[0.7.0]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.6.3...rusk-abi-0.7.0
[0.6.3]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.6.2...rusk-abi-0.6.3
[0.6.2]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.6.0...rusk-abi-0.6.2
[0.6.0]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.5.0...rusk-abi-0.6.0
[0.5.0]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.4.1...rusk-abi-0.5.0
[0.4.1]: https://github.com/dusk-network/dusk-abi/compare/rusk-abi-0.4.0...rusk-abi-0.4.1
[0.4.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.4.0
[0.3.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.3.0
[0.2.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.2.0
[0.1.0]: https://github.com/dusk-network/dusk-abi/releases/tag/rusk-abi-0.1.0
