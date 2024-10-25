# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Add

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
- Change Rusk wallet name and version information [#2647]

### Fix

- Fix stake info for inactive stakes with rewards [#2766]
- Fix Moonlight stake reward withdrawal [#2523]

[#2766]: https://github.com/dusk-network/rusk/issues/2766
[#2708]: https://github.com/dusk-network/rusk/issues/2708
[#2702]: https://github.com/dusk-network/rusk/issues/2702
[#2659]: https://github.com/dusk-network/rusk/issues/2659
[#2647]: https://github.com/dusk-network/rusk/issues/2647
[#2566]: https://github.com/dusk-network/rusk/issues/2566
[#2523]: https://github.com/dusk-network/rusk/issues/2523
[#2402]: https://github.com/dusk-network/rusk/issues/2402
[#2401]: https://github.com/dusk-network/rusk/issues/2401
[#2400]: https://github.com/dusk-network/rusk/issues/2400
[#2396]: https://github.com/dusk-network/rusk/issues/2396
[#2340]: https://github.com/dusk-network/rusk/issues/2340
[#2288]: https://github.com/dusk-network/rusk/issues/2288

<!-- Releases -->
[unreleased]: https://github.com/dusk-network/rusk/compare/master
<!-- [0.1.0]: https://github.com/dusk-network/rusk/releases/tag/rusk-wallet-0.1.0 -->
