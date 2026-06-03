# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add public `BlobError`, `ConversionError`, `RuskError`, `TransactionError`, and `ProverError` error types, re-exported from `rusk_wallet`
- Add `Error::InvalidEndpointUrl` and `Error::UnsupportedEndpointScheme` variants for endpoint validation failures
- Show the stake owner in the TUI stake info view

### Changed

- Share CLI/TUI balance and operation result models.
- Treat `0.0.0.0` and `::` HTTP endpoints as insecure unless `--allow-insecure` is set
- Reject endpoint URLs with unsupported schemes instead of accepting non-HTTP(S) values during settings validation
- Change `Error::{Rusk, Transaction, Blob, Conversion, ProverError}` to carry structured sub-enums instead of `String` payloads
- Replace `Error::ArchiveJsonError(String)` with `Error::ArchiveQuery(Box<Error>)` and `Error::ArchiveJson(serde_json::Error)`
- Initialize logging from CLI flags before loading config so config parse/setup errors are captured by the selected logger
- Warn when creating or writing the default config fails before falling back to embedded defaults

### Fixed

- Reject malformed `--state`, `--prover`, and `--archiver` URL overrides instead of silently falling back to config values
- Update the TUI stake amount max when switching between shielded and public staking
- Restore TUI stake owner profile selection
- Show an error when changing the TUI stake owner without another profile
- Lock the TUI stake owner selection when topping up an existing stake

## [0.4.0] - 2026-05-07

### Added

- Select the active protocol transaction format when encoding propagated
  transactions and decoding archived transaction bytes.
- Add full-screen TUI mode as the default interactive wallet experience
- Add startup sync gate screen in TUI with cycle stage, progress bar, status stream, and block-height feedback
- Add chain tip height polling and display in TUI overview status
- Add explicit network indicator in TUI overview, using configured network name (including custom `--network` names)
- Add `View Addresses` dashboard action to show full shielded/public addresses in TUI

### Changed

- Stop selecting propagated transaction envelopes from local hardfork height and use the stable client network encoding instead
- Improve TUI startup responsiveness by avoiding long blocking phases on initial sync
- Stabilize sync status transitions to reduce rapid `Synced`/`Syncing` toggling around normal block cadence
- Rename dashboard action to `Import Different Wallet` and clarify that import replaces the current wallet (backup kept as `wallet.dat.old`)
- Parallelize Phoenix note ownership scanning during sync across available logical cores
- Increase default transfer gas limit to `50_000_000` and split built-in wallet actions into a `150_000_000` gas bucket for Boreas-era transaction costs
- Delegate contract-id derivation to the canonical `dusk-core` deployment helper

### Fixed

- Decode archived GraphQL transaction history by the stored envelope instead of the local hardfork schedule
- Derive Rues contract entity paths from `ContractId` bytes instead of hard-coding transfer/stake IDs
- Route wallet GraphQL queries for propagation format detection through the
  canonical `/graphql` endpoint, removing the remaining dependency on the
  legacy `/on/graphql/query` route
- Restore the wallet setup option in interactive to create a new wallet when no wallet exists or when importing a different wallet
- Refuse remote `http://` wallet service endpoints unless `--allow-insecure` is set, while keeping loopback HTTP available for local development
- Batch `existing_nullifiers` sync queries to avoid large restore failures
- Detect stale note cache (e.g., from a wiped local node) and reset it automatically before syncing
- Zeroize the validated mnemonic phrase immediately after CLI restore derives the wallet
- Create CLI mnemonic seed files with owner-only permissions on Unix
- Fix TUI stdout artifacting and stale frame residue after startup sync
- Ignore placeholder `block 0` sync values so invalid heights are not shown in overview
- Restore pre-submit balance checks in TUI command flow for clearer insufficient-balance feedback
- Fix new-wallet password screen cursor to follow the active input field
- Harden wallet prompt/TUI secret handling and terminal cleanup paths
- Fix TUI import/restore lock contention by closing the active wallet before importing a different one
- Clear stale cache when importing a different wallet and auto-retry connect on cache schema mismatch errors
- Avoid printing raw startup/offline connection warnings into the terminal buffer while TUI is active
- Fix Claim Reward info about the max amount & show `max: Unknown` when stake reward lookup fails
- Refresh TUI stake reward display after confirmed stake, unstake, and claim-rewards transactions

## [0.3.0] - 2026-02-27

### Changed

- Update to edition 2024
- Replace `serde_as` with stable `serde(with)` attributes
- Update MSRV to 1.85

### Added

- Add global config hierarchy with auto-creation at `~/.config/rusk-wallet/` [#2518]
- Add config to change the default wallet directory [#3775]
- Add support for blob transaction
- Add interactive status output [#3397]
- Add check to ensure the wallet can cover max fee in interactive mode [#3695]

### Changed

- Change transaction history action names to be more descriptive [#3801]
- Improve error message when querying history on non-archive node [#3977]
- Change withdraw rewards to withdraw partial amount [#2538]
- Change transaction history fee display to be negative
- Change transaction history item type to display public/shielded
- Separate archive node endpoint from state endpoint
- Change withdraw command to claim rewards [#3077]
- Ensure zeroize is called for secret info [#3687]
- Change default transfer gas limit to a safer value [#3948]
- Add staking and contract submenus to interactive mode [#3645]

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
[#3801]: https://github.com/dusk-network/rusk/issues/3801
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
[#3645]: https://github.com/dusk-network/rusk/issues/3645
[#3623]: https://github.com/dusk-network/rusk/issues/3623
[#3602]: https://github.com/dusk-network/rusk/issues/3602
[#3600]: https://github.com/dusk-network/rusk/issues/3600
[#3598]: https://github.com/dusk-network/rusk/issues/3598
[#3597]: https://github.com/dusk-network/rusk/issues/3597
[#3593]: https://github.com/dusk-network/rusk/issues/3593
[#3405]: https://github.com/dusk-network/rusk/issues/3405
[#3397]: https://github.com/dusk-network/rusk/issues/3397
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
[Unreleased]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.4.0...HEAD
[0.4.0]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.3.0...rusk-wallet-0.4.0
[0.3.0]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.2.0...rusk-wallet-0.3.0
[0.2.0]: https://github.com/dusk-network/rusk/compare/rusk-wallet-0.1.0...rusk-wallet-0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/tree/rusk-wallet-0.1.0
