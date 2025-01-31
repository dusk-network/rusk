# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Change dependency declaration to not require strict equal [#3405]

### Fix

- Fix wrong lower limit for stake operation when performing topup [#3394]

## [0.1.0] - 2025-01-20

### Add

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

### Fix

- Fix phoenix balance update [#2488]
- Fix stake info for inactive stakes with rewards [#2766]
- Fix Moonlight stake reward withdrawal [#2523]


<!-- Issues -->
[#3405]: https://github.com/dusk-network/rusk/issues/3405
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
[#3394]: https://github.com/dusk-network/rusk/issues/3394

<!-- Releases -->
[Unreleased]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/rusk/tree/rusk-wallet-0.1.0
