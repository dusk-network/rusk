# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add deploy contract output (display the new contractId)
- Add optional deposit to ContractCall [#3650]
- Add pagination for transaction history to not pollute the stdout [#3292]

### Changed

- Change dependency declaration to not require strict equal [#3405]
- Change key derivation to PBKDF2 and wallet encryption to AES-GCM [#3391]
- Change default deploy gas limit to be accepted by std mempool

### Removed

- Remove `async` from `State::register_sync` [#3623]
- Remove return of `Result<(), Error>` from `State::register_sync` [#3623]

### Fixed

- Fix wrong lower limit for stake operation when performing topup [#3394]
- Fix `is_synced()` method in the wallet to avoid overflow [#3593]
- Fix transaction history deserialization [#3598]
- Fix contract init parsing [#3602]
- Fix contract call non-interactive parsing [#3602]
- Fix exit when command errors in interactive mode [#3597], [#3192]
- Fix out of order transaction history [#3702]

## [0.1.0] - 2025-01-20

### Added

- Add gas cost calculation to contract deploy [#2768]
- Add more information to `stake-info` [#2659]
- Add string length validation to memo transfer and function calls [#2566]
- Add contract deploy and contract calling [#2402]
- Add support for RUES [#2401]
- Add Moonlight stake, unstake and withdraw [#2400]
- Add balance validation for any given transaction action [#2396]
- Add Moonlight-Phoenix conversion [#2340]
- Add Moonlight transactions [#2288]

### Changed

- Changed default gas limits
- Split `prove_and_propagate` into `prove` and `propagate` [#2708]
- Unify `sndr_idx` and `profile_idx` fields in `Command` enum [#2702]
- Rename `--profile` flag to `--wallet-dir` [#2682]
- Change Rusk wallet name and version information [#2647]
- Update Clap from v3 to workspace v4 [#2489]
- Rename all instances of recovery phrase to mnemonic phrase [#2839]
- Rename Shielded account to be aligned with the Web wallet [#3263]

### Fixed

- Fix phoenix balance update [#2488]
- Fix stake info for inactive stakes with rewards [#2766]
- Fix Moonlight stake reward withdrawal [#2523]


<!-- Issues -->
[#3702]: https://github.com/dusk-network/rusk/issues/3702
[#3597]: https://github.com/dusk-network/rusk/issues/3597
[#3192]: https://github.com/dusk-network/rusk/issues/3192
[#3650]: https://github.com/dusk-network/rusk/issues/3650
[#3623]: https://github.com/dusk-network/rusk/issues/3623
[#3602]: https://github.com/dusk-network/rusk/issues/3602
[#3598]: https://github.com/dusk-network/rusk/issues/3598
[#3593]: https://github.com/dusk-network/rusk/issues/3593
[#3405]: https://github.com/dusk-network/rusk/issues/3405
[#3394]: https://github.com/dusk-network/rusk/issues/3394
[#3391]: https://github.com/dusk-network/rusk/issues/3391
[#3292]: https://github.com/dusk-network/rusk/issues/3292
[#3263]: https://github.com/dusk-network/rusk/issues/3263
[#2839]: https://github.com/dusk-network/rusk/issues/2839
[#2768]: https://github.com/dusk-network/rusk/issues/2768
[#2766]: https://github.com/dusk-network/rusk/issues/2766
[#2708]: https://github.com/dusk-network/rusk/issues/2708
[#2702]: https://github.com/dusk-network/rusk/issues/2702
[#2682]: https://github.com/dusk-network/rusk/issues/2682
[#2659]: https://github.com/dusk-network/rusk/issues/2659
[#2647]: https://github.com/dusk-network/rusk/issues/2647
[#2566]: https://github.com/dusk-network/rusk/issues/2566
[#2523]: https://github.com/dusk-network/rusk/issues/2523
[#2489]: https://github.com/dusk-network/rusk/issues/2489
[#2488]: https://github.com/dusk-network/rusk/issues/2488
[#2402]: https://github.com/dusk-network/rusk/issues/2402
[#2401]: https://github.com/dusk-network/rusk/issues/2401
[#2400]: https://github.com/dusk-network/rusk/issues/2400
[#2396]: https://github.com/dusk-network/rusk/issues/2396
[#2340]: https://github.com/dusk-network/rusk/issues/2340
[#2288]: https://github.com/dusk-network/rusk/issues/2288

<!-- Releases -->
[Unreleased]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/rusk/tree/rusk-wallet-0.1.0
