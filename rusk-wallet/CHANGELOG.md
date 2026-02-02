# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add global config hierarchy with auto-creation at `~/.config/rusk-wallet/` [#2518]
- Add config to change the default wallet directory [#3775]
- Add support for blob transaction
- Add check to ensure the wallet can cover max fee in interactive mode [#3695]

### Changed

- Improve error message when querying history on non-archive node [#3977]
- Change withdraw rewards to withdraw partial amount [#2538]
- Change transaction history fee display to be negative
- Change transaction history item type to display public/shielded
- Separate archive node endpoint from state endpoint
- Change withdraw command to claim rewards [#3077]
- Ensure zeroize is called for secret info [#3687]
- Change default transfer gas limit to a safer value [#3948]

### Fixed

- Fix transaction history error when the wallet has no stake [#3734]
- Fix transaction history fail after shield/unshield conversions [#3600]
- Fix transaction history fail after stake/unstake [#3712]
- Fix inconsistent navigation and exiting [#3792]

## [0.2.0] - 2025-05-07

### Added

- Add deploy contract output (display the new contractId)
- Add optional deposit to ContractCall [#3650]
- Add pagination for transaction history to not pollute the stdout [#3292]

### Changed

- Change dependency declaration to not require strict equal [#3405]
- Change key derivation to PBKDF2 and wallet encryption to AES-GCM [#3391]
- Change default deploy gas limit to be accepted by std mempool
- Change transaction history error message to a more helpful one [#3707]

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
- Fix empty transaction history error [#3700]
- Fix insufficient balance to stake error message [#3713]
- Fix wrong amount in phoenix transaction history [#3704]

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
[#3977]: https://github.com/dusk-network/rusk/issues/3977
[#3948]: https://github.com/dusk-network/rusk/issues/3948
[#3792]: https://github.com/dusk-network/rusk/issues/3792
[#3775]: https://github.com/dusk-network/rusk/issues/3775
[#3077]: https://github.com/dusk-network/rusk/issues/3077
[#3734]: https://github.com/dusk-network/rusk/issues/3734
[#3713]: https://github.com/dusk-network/rusk/issues/3713
[#3712]: https://github.com/dusk-network/rusk/issues/3712
[#3707]: https://github.com/dusk-network/rusk/issues/3707
[#3704]: https://github.com/dusk-network/rusk/issues/3704
[#3702]: https://github.com/dusk-network/rusk/issues/3702
[#3700]: https://github.com/dusk-network/rusk/issues/3700
[#3695]: https://github.com/dusk-network/rusk/issues/3695
[#3687]: https://github.com/dusk-network/rusk/issues/3687
[#3650]: https://github.com/dusk-network/rusk/issues/3650
[#3623]: https://github.com/dusk-network/rusk/issues/3623
[#3602]: https://github.com/dusk-network/rusk/issues/3602
[#3600]: https://github.com/dusk-network/rusk/issues/3600
[#3598]: https://github.com/dusk-network/rusk/issues/3598
[#3597]: https://github.com/dusk-network/rusk/issues/3597
[#3593]: https://github.com/dusk-network/rusk/issues/3593
[#3405]: https://github.com/dusk-network/rusk/issues/3405
[#3394]: https://github.com/dusk-network/rusk/issues/3394
[#3391]: https://github.com/dusk-network/rusk/issues/3391
[#3292]: https://github.com/dusk-network/rusk/issues/3292
[#3263]: https://github.com/dusk-network/rusk/issues/3263
[#3192]: https://github.com/dusk-network/rusk/issues/3192
[#2839]: https://github.com/dusk-network/rusk/issues/2839
[#2768]: https://github.com/dusk-network/rusk/issues/2768
[#2766]: https://github.com/dusk-network/rusk/issues/2766
[#2708]: https://github.com/dusk-network/rusk/issues/2708
[#2702]: https://github.com/dusk-network/rusk/issues/2702
[#2682]: https://github.com/dusk-network/rusk/issues/2682
[#2659]: https://github.com/dusk-network/rusk/issues/2659
[#2647]: https://github.com/dusk-network/rusk/issues/2647
[#2566]: https://github.com/dusk-network/rusk/issues/2566
[#2538]: https://github.com/dusk-network/rusk/issues/2538
[#2523]: https://github.com/dusk-network/rusk/issues/2523
[#2518]: https://github.com/dusk-network/rusk/issues/2518
[#2489]: https://github.com/dusk-network/rusk/issues/2489
[#2488]: https://github.com/dusk-network/rusk/issues/2488
[#2402]: https://github.com/dusk-network/rusk/issues/2402
[#2401]: https://github.com/dusk-network/rusk/issues/2401
[#2400]: https://github.com/dusk-network/rusk/issues/2400
[#2396]: https://github.com/dusk-network/rusk/issues/2396
[#2340]: https://github.com/dusk-network/rusk/issues/2340
[#2288]: https://github.com/dusk-network/rusk/issues/2288

<!-- Releases -->
[Unreleased]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.2.0...HEAD
[0.2.0]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.1.0...rusk-wallet-0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/rusk-wallet-0.1.0
