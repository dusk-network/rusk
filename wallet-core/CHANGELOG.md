# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add `unstake` function to allow unstaking a previously staked amount [#58]
- Add `fetch_existing_nullifiers` to the `StateClient` [#41]

### Changed

- Change FFI to take pointers to `u64`
- Change `withdraw` function to withdraw the reward for staking and
  participating in the consensus [#58]
- Change `stake` and `withdraw` cryptographic signatures to what the stake
  contract expects in the new spec [#58]
- Change `StakeInfo` to have an optional amount staked, a reward, and a
  signature counter [#58]
- Change signature of `fetch_notes` by removing block height
- Change `fetch_notes` in the FFI to delegate buffer allocation to the user. [#58]
- Change note picker to pick the largest amount of notes possible, with the minimum
  total value [#55]
- Change `get_balance` to return total and max spendable [#53]
- Change `fetch_existing_nullifiers` in the FFI to return early [#49]
- Modify `StateClient` to receive full stake info [#46]
- Change `Canon` encoding length of `Transaction` [#31]
- Change transacting functions to return `Transaction` [#40]
- Update transaction to generate change output
- Change transactions to have an optional crossover
- Update STCT to use Schnorr signatures [#34]
- Update dependencies

### Fixed

- Fix `encoded_len` in `Transaction` [#44]
- Fix notes from `fetch_notes` being assumed unspent [#41]
- Fix fee generation in stake and withdrawal

### Removed

- Remove `get_block_height` from the `StateClient` trait [#58]
- Delete `extend_stake` since the stake contract removed it [#46]

## [0.5.1] - 2021-01-26

### Fixed

- Stack overflow by allocating `fetch_notes` buffer on the heap [#25]

## [0.5.0] - 2021-01-25

### Changed

- Change stake operations to use BLS keys instead of JubJub keys [#22]

## [0.4.0] - 2021-01-24

### Changed

- Update `rusk-abi` [#20]

## [0.3.0] - 2021-01-23

### Added

- Implemented `Canon` for `Transaction` [#16]

### Changed

- Rename `to_bytes` to `to_var_bytes` when the struct is variable sized [#17]

## [0.2.1] - 2021-01-22

### Added

- Add publishing built WASM to Github NPM [#1]
- Add `malloc` and `free` to FFI [#13]

## [0.2.0] - 2021-01-20

### Added

- Implemented `stake` and `withdraw` [#6]

## [0.1.0] - 2021-01-13

### Added

- Serialization and deserialization of transactions
- Temporary deterministic wallet generation
- Implementation of `get_balance` and `public_spend_key`
- Preliminary implementation of `create_transfer_tx`
- Expose `NodeClient` and `Store` through FFI
- Define FFI and compile it only for WASM

[#58]: https://github.com/dusk-network/wallet-core/issues/58
[#55]: https://github.com/dusk-network/wallet-core/issues/55
[#53]: https://github.com/dusk-network/wallet-core/issues/53
[#49]: https://github.com/dusk-network/wallet-core/issues/49
[#46]: https://github.com/dusk-network/wallet-core/issues/46
[#44]: https://github.com/dusk-network/wallet-core/issues/44
[#41]: https://github.com/dusk-network/wallet-core/issues/41
[#40]: https://github.com/dusk-network/wallet-core/issues/40
[#34]: https://github.com/dusk-network/wallet-core/issues/34
[#31]: https://github.com/dusk-network/wallet-core/issues/31
[#25]: https://github.com/dusk-network/wallet-core/issues/25
[#22]: https://github.com/dusk-network/wallet-core/issues/22
[#20]: https://github.com/dusk-network/wallet-core/issues/20
[#17]: https://github.com/dusk-network/wallet-core/issues/17
[#16]: https://github.com/dusk-network/wallet-core/issues/16
[#13]: https://github.com/dusk-network/wallet-core/issues/13
[#6]: https://github.com/dusk-network/wallet-core/issues/6
[#1]: https://github.com/dusk-network/wallet-core/issues/1

<!-- Releases -->

[Unreleased]: https://github.com/dusk-network/wallet-core/compare/v0.5.1...HEAD
[0.5.1]: https://github.com/dusk-network/wallet-core/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/dusk-network/wallet-core/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/dusk-network/wallet-core/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/dusk-network/wallet-core/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/dusk-network/wallet-core/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/dusk-network/wallet-core/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dusk-network/wallet-core/releases/tag/v0.1.0
