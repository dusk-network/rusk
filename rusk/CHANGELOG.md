# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.3.0] - 2025-04-17

### Added

- Add `Content-Type: application/json` support for `/on/contracts` endpoint
- Add transaction serialization check
- Add max transaction size check
- Add `/on/driver:<contract>/<method>:<target>` endpoint
- Add from_block & to_block params to `full_moonlight_history` in gql [#3613]

### Changed

- Upgrade piecrust to `0.28.1`

## [1.2.0] - 2025-03-20

### Added

- Add simulate transaction API [#1225]

### Changed

- Change plonk verification to use embed verification data by default [#3507]
- Change responses for moonlight gql endpoints (archive node) [#3512]
- Change `prover` feature to include `recovery-keys` feature [#3507]
- Change `piecrust` dependency to `0.28.0`

## [1.1.1] - 2025-02-21

### Changed

- Upgrade piecrust to `0.27.2`

## [1.1.0] - 2025-02-14

### Added

- Add `abi::public_sender` [#3341]
- Add `[vm]` config section [#3341]
- Add CONTRACT_TO_ACCOUNT inflow case on archive moonlight filtering [#3494]
- Add Dockerfile for persistent state builds [#1080]
- Add `vm_config` section to `/on/node/info` [#3341]

### Changed

- Deprecate `[chain].gas_per_deploy_byte` config [#3341]
- Deprecate `[chain].min_deployment_gas_price` config [#3341]
- Deprecate `[chain].generation_timeout` config [#3341]
- Deprecate `[chain].min_deploy_points` config [#3341]
- Deprecate `[chain].block_gas_limit` config [#3341]
- Change how Rusk controls the archive for synchronization [#3359]
- Update `bls12_381-bls` to 0.5 [#2773]
- Update `dusk-bls12_381` to 0.14 [#2773]
- Update `dusk-jubjub` to 0.15.0 [#2773]
- Update `dusk-plonk` to 0.21.0 [#2773]
- Update `dusk-poseidon` to 0.41 [#2773]
- Update `jubjub-schnorr` to 0.6 [#2773]
- Update `phoenix-circuits` to 0.6 [#2773]
- Update `phoenix-core` to 0.34.0 [#2773]
- Update `poseidon-merkle` to 0.8 [#2773]

### Fixed

- Fix node unresponsiveness when querying contracts that take too long to terminate [#3481]

### Removed

- Remove legacy event system 
- Remove archive mpsc channel & archive event forwarding [#3359]

## [1.0.2] - 2025-01-27

### Added

- Add `/on/account:<address>/status` endpoint [#3422]

### Changed

- Change dependency declaration to not require strict equal [#3405]

## [1.0.1] - 2025-01-20

### Added

- Add `Rusk-Version-Strict` header for version match
- Add support for `cargo install`

### Changed

- Change `check_rusk_version` to ignore pre-release by default
- Increase archive channel capacity from 1000 to 10000 [#3359]
- Change `/static/drivers/wallet-core.wasm` to embed version `1.0.0`

## [1.0.0] - 2025-01-05

### Added

- Add support for configurable HTTP headers [#2480]
- Add serialization to GraphQL error returned to clients [#2423]
- Add preliminary archive GraphQL endpoints [#2583]
- Add .editorconfig to repository [#2802]
- Add network-trace feature [#3243]
- Add new feature & change Dockerfile [#3030]
- Add test for finalized roots [#3052]
- Add Moonlight account to genesis [#2570]
- Add StakeFundOwner info to API [#3193]

### Changed

- Change voters to be `Vec` instead of `Option` [#2457]
- Change genesis_timestamp to be optional [#2473]
- Change rusk-provisioners to filter out slashed stakes [#1257]
- Update README header [#2909]
- Upgrade protocol version [#2720]
- Refactor CLI subcommands [#2479]
- Refactor rusk builder [#2415]
- Refactor event loop for improved performance [#3157]

### Fixed

- Fix HTTP preverification [#2495]
- Fix binary request detection in RUES [#2379]
- Fix edge case where gas_spent result higher than gas_limit [#2591]
- Fix finalize_state [#2592]
- Fix genesis_timestamp to be optional [#2473]
- Fix sender JSON representation [#2610]
- Fix geoip endpoint [#2671]
- Fix gql check_block endpoint [#2910]
- Fix mempool stack overflow [#3209]
- Fix contract deploy properly serialize init args [#3090]
- Fix `max_value` computation to handle overflow error [#3206]

### Removed

- Remove conservative generator strategy [#2360]
- Remove dirty state folders [#2392]

## [0.8.0] - 2024-09-10

### Added

- Add mempool transaction replacement [#1271]
- Add chain events [#2049]
- Add protocol version to Message [#2251]
- Add implementation for RUES dispatch [#2224]
- Add HTTP endpoint `/static/drivers/wallet-core.wasm` [#2268]
- Add moonlight support in rusk-wallet [#2289]
- Enrich contract events with transaction ID [#2296]
- Add `min_gas_limit` node configuration argument and enforcement [#2597]
- Add `gen_contract_id` and 32-byte hash for contract deployment [#1884]
- Add execution of contract deployment [#1882]
- Add first version of RUES, allowing websocket clients to subscribe for events emitted by block transitions [#931]
- Add `ws_sub_channel_cap` and `ws_sub_channel_cap` configuration items, allowing for roughly regulating the throughput websockets connections [#931]
- Add `base64` dependency [#931]
- Add type constraints for bytecheck [#1371]
- Add TLS support for HTTP server
- Add iteration generator to FailedIterations [#1257]
- Add `node` feature flag [#1144]
- Add `RUSK_CRS_URL` environment variable

### Changed

- Allow rusk to be run as prover only [#1376]
- Exit on invalid configuration [#1481]
- Upgrade kadcast version [#2218]
- Implement new emission schedule [#2274]
- Adapt emission schedule to emit 500M dusk [#2283]
- Enrich provisioners info with contract-related events [#2294]
- Refactor rusk binary [#2047]
- Ported to Piecrust 0.25.0 [#2536]
- Allow state transitions to be executed in parallel with queries [#970]
- Change dependencies declarations to enforce bytecheck [#1371]
- Fixed tests passing incorrect arguments [#1371]
- Adjusted deployment charging [#2207]
- Adapt accept/finalize state transitions to broadcast emitted events through RUES [#931]
- Change rusk::provisioners to filter out slashed stakes [#1257]
- Change block processing to slash failed-iteration provisioners [#1257]
- Change FailedIterations to include only nil quorum certificates [#1257]
- Extended block generator with support for economic gas handling [#1603]

### Fixed

- Fix MacOS HTTPS test [#1275]
- Fix corrupted state after restart [#1640]
- Fix ephemeral state folder [#1870]
- Fix AST benchmarks [#1959]
- Fix compilation with memo updates in rusk-wallet [#2282]
- Fix prev_stake for Stake operation in stake-contract [#2299]
- Fix RUES handling of WS `Message::Close` [#2341]
- Fix corrupted state after restart [#1640]

### Removed

- Remove allowlist [#1257]
- Remove unused dependencies [#1885]
- Remove economic protocol handling
- Remove STCO and WFCO [#1675]

## [0.7.0] - 2023-12-31

### Added

- Add gas price API: max, min and mean gas prices
- Add rolling finality
- Add AST benchmarks
- Add common module to benchmark
- Add criterion dependency for benchmarks
- Add `prover` feature flag as default
- Add hash header to the CRS-via-HTTP api
- Add support for HTTP response headers
- Add test for unspendable transaction
- Add HTTP `crs` topic
- Add HTTP support for explicit binary response
- Add `rusk-recovery` subcommands
- Add `info` HTTP endpoint
- Add HTTP support for `application/json` content type
- Add HTTP version handshake
- Add `network_id` config
- Add `build` recipe to Makefile
- Add `--profile` arg
- Add GQL mempool API
- Add `alive_nodes` api
- Add `provisioners` api
- Add GQL block state hash field
- Add blocks by range graphql handler
- Add transactions by block range graphql handler
- Add tests for HTTP handler
- Add `HandleRequest` trait
- Add `propagate_tx` http handler
- Add `prove_*` http handler
- Add `preverify` http handler
- Add `MessageRequest::to_error`
- Add http format request
- Add support for gql variables
- Add various gql queries
- Add support for binary http requests
- Add handler for query_raw
- Add basic graphql implementation
- Add `multi_transfer` test
- Add `gas_behaviour` test
- Add `stake` test
- Add `wallet` test (aka `wallet_grpc`)
- Add `get_notes` functionality
- Add `preverify` implementation of node's trait
- Add client request parsing
- Add default for WsConfig
- Add listening for incoming websocket streams
- Add node dependency

### Changed

- Change `dusk` key
- Change `finalize` to not remove previous vm commit
- Change `Provisioners::default()` to `Provisioners::empty()`
- Change http `prover` to activate behind feature flag
- Change `clippy` to cover all features
- Change `stake.toml` configuration for tests
- Change error messages while accepting blocks
- Change VST to use candidate gas_limit
- Change BlockGenerator to produce the correct gas_limit value
- Change gql dep to `async-graphql`
- Change external event system implementation
- Change `consistency_check` to `Option`
- Change codebase to support new `SpentTransaction`

### Removed

- Remove obsoleted test
- Remove multiple stakes from provisioner member
- Remove timing for subcommands
- Remove `RUSK_PROFILE_PATH` dependency
- Remove `rusk-recovery` as library dependency.
- Remove tracing prefix
- Remove unnecessary quotes from text response
- Remove `Ws` prefix for `http` module structs
- Remove manual async executor instantiation
- Remove/disable deprecated tests
- Remove dependencies from binary
- Remove obsolete dependencies
- Remove `rusk-schema` dependency
- Remove grpc server

### Fixed

- Fix binary HTTP event detection
- Fix HTTP header parsing for unquoted strings
- Fix HTTP header parsing for empty values
- Fix default HTTP_Listener
- Fix graphql latest transaction filter
- Fix http listener activation
- Fix double execution for incoming request
- Fix gas spent for ICC
- Fix json response serialization
- Fix Event::parse
- Fix routing for Host("rusk")

## [0.6.0] - 2023-06-21

## [0.5.3] - 2022-03-21

## [0.4.0] - 2021-08-13

## [0.3.0] - 2021-07-15

### Added
- Prover service implementation [#410]

### Changed

- Update `dusk-poseidon` to `0.22.0-rc` [#327]
- Update `dusk-pki` to `0.8.0-rc` [#327]
- Update `phoenix-core` to `0.13.0-rc` [#327]
- Update `dusk-schnorr` to `0.8.0-rc` [#327]
- Update the blindbid service to use the new API of `dusk-blindbid` [#327]

### Removed

- Remove bid contracts and circuits [#369]
- Remove `dusk-blindbid` crate and rusk's blindbid services [#369]

## [0.2.0] - 2021-05-19

### Added

- Add setup/teardown system for tests [#292]
- Add `dusk-jubjub v0.10` to deps [#292]
- Add `async-stream` to deps [#292]
- Add `test-context v0.1` to dev-dependencies [#292]
- Add `async-trait v0.1` to dev-dependencies [#292]
- Add `RUSK_PROFILE_PATH` env variable check in `build.rs` [#307]

### Changed

- Update build system improving Circuit keys caching [#290]
- Update `tokio` to `v1.6` [#292]
- Update `tonic` from `0.3` to `0.4` [#292]
- Update `prost` from `0.6` to `0.7` [#292]
- Change `tower` to be a dev-dependency [#292]
- Refactor `unix` modules from `tests` and `bin` [#292]

### Fixed

- Fix dusk-bytes encoding issues [#292]
- Fix score generation module/service [#292]

### Removed

- Remove `bincode` since is unused [#292]
- Remove `default-feats = false` from `dusk-plonk` [#292]
- Remove `thiserror` from dependencies [#292]

## [0.1.0] - 2021-02-19

### Added

- Add Rusk server impl
- Add BlindBid service
- Add Pki service
- Add Echo service
- Add encoding module
- Add clap cli interface
- Add linking between Rusk and Protobuff structs
- Add build system that generates keys for circuits and caches them.

<!-- Issues -->
[#3613]: https://github.com/dusk-network/rusk/issues/3613
[#3512]: https://github.com/dusk-network/rusk/issues/3512
[#3507]: https://github.com/dusk-network/rusk/issues/3507
[#3494]: https://github.com/dusk-network/rusk/issues/3494
[#3481]: https://github.com/dusk-network/rusk/issues/3481
[#3359]: https://github.com/dusk-network/rusk/issues/3359
[#3422]: https://github.com/dusk-network/rusk/issues/3422
[#3405]: https://github.com/dusk-network/rusk/issues/3405
[#3341]: https://github.com/dusk-network/rusk/issues/3341
[#3359]: https://github.com/dusk-network/rusk/issues/3359
[#3206]: https://github.com/dusk-network/rusk/issues/3206
[#2773]: https://github.com/dusk-network/rusk/issues/2773
[#2597]: https://github.com/dusk-network/rusk/issues/2597
[#2536]: https://github.com/dusk-network/rusk/issues/2536
[#2207]: https://github.com/dusk-network/rusk/issues/2207
[#1884]: https://github.com/dusk-network/rusk/issues/1884
[#1882]: https://github.com/dusk-network/rusk/issues/1882
[#1675]: https://github.com/dusk-network/rusk/issues/1675
[#1640]: https://github.com/dusk-network/rusk/issues/1640
[#1603]: https://github.com/dusk-network/rusk/issues/1603
[#1371]: https://github.com/dusk-network/rusk/issues/1371
[#1257]: https://github.com/dusk-network/rusk/pull/1257
[#1225]: https://github.com/dusk-network/rusk/issues/1225
[#1219]: https://github.com/dusk-network/rusk/issues/1219
[#1144]: https://github.com/dusk-network/rusk/issues/1144
[#1080]: https://github.com/dusk-network/rusk/issues/1080
[#970]: https://github.com/dusk-network/rusk/issues/970
[#931]: https://github.com/dusk-network/rusk/issues/931
[#401]: https://github.com/dusk-network/rusk/issues/401
[#369]: https://github.com/dusk-network/rusk/issues/369
[#327]: https://github.com/dusk-network/rusk/issues/327
[#307]: https://github.com/dusk-network/rusk/issues/307
[#292]: https://github.com/dusk-network/rusk/issues/292
[#290]: https://github.com/dusk-network/rusk/issues/290


[Unreleased]: https://github.com/dusk-network/rusk/compare/dusk-rusk-1.3.0...HEAD
[1.3.0]: https://github.com/dusk-network/rusk/compare/dusk-rusk-1.2.0...dusk-rusk-1.3.0
[1.2.0]: https://github.com/dusk-network/rusk/compare/dusk-rusk-1.1.1...dusk-rusk-1.2.0
[1.1.1]: https://github.com/dusk-network/rusk/compare/dusk-rusk-1.1.0...dusk-rusk-1.1.1
[1.1.0]: https://github.com/dusk-network/rusk/compare/dusk-rusk-1.0.2...dusk-rusk-1.1.0
[1.0.2]: https://github.com/dusk-network/rusk/compare/rusk-1.0.1...dusk-rusk-1.0.2
[1.0.1]: https://github.com/dusk-network/rusk/compare/rusk-1.0.0...rusk-1.0.1
[1.0.0]: https://github.com/dusk-network/rusk/compare/v0.8.0...rusk-1.0.0
[0.8.0]: https://github.com/dusk-network/rusk/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/dusk-network/rusk/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/dusk-network/rusk/compare/v0.5.3...v0.6.0
[0.5.3]: https://github.com/dusk-network/rusk/compare/v0.4.0...v0.5.3
[0.4.0]: https://github.com/dusk-network/rusk/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/dusk-network/rusk/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/dusk-network/rusk/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dusk-network/rusk/releases/tag/rusk-0.1.0
