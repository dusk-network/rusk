# CLAUDE.md

## Project Overview

**Rusk** is the Dusk node client and smart contract stack. Rust workspace + JS/TS/Deno projects for wallets, SDKs, and UI.

### Repo Map

| Directory | Purpose |
|-----------|---------|
| `rusk/` | Node binary, HTTP/GraphQL APIs, VM/chain/prover glue |
| `node/` | Chain node (networking, mempool, storage) |
| `node-data/` | Ledger types, network messages (serialization-sensitive) |
| `consensus/` | Succinct Attestation consensus |
| `vm/` | WASM contract VM (Piecrust) + host queries |
| `contracts/` | Genesis contracts (`stake`, `transfer`) + test contracts |
| `rusk-profile/` | Circuit artifacts, genesis profiles |
| `rusk-prover/` | PLONK prover implementation |
| `rusk-recovery/` | State/key recovery utilities |
| `wallet-core/` | Wallet primitives, WASM/FFI surface |
| `rusk-wallet/` | CLI wallet |
| `data-drivers/` | RKYV ↔ JS bridge for contract calls |
| `w3sper.js/` | Deno SDK |

Note: the web wallet lives in the external repo `dusk-network/web-wallet`.

## Commands

### Setup

```bash
bash scripts/dev-setup.sh      # System deps + Rust toolchain
make setup-compiler            # Dusk contract compiler (cargo +dusk)
```

### Build

```bash
make                           # Everything
cargo build -p dusk-rusk       # Node binary (add --release for prod)
cargo check -p <crate>         # Fast compile check
```

### Test

```bash
make test                              # Full suite (slow)
cargo test -p <crate> --release        # Single crate
make -C <dir> test                     # Crate via Makefile
```

### Lint

```bash
make clippy                    # All crates (warnings = errors)
cargo fmt --all                # Format
```

### PR Minimum

```bash
cargo test -p <package> --release
make -C <dir> clippy
```

Expand for widely-depended packages (`core/`, `node-data/`) or elevated care zones.

### Contracts

```bash
make wasm                      # All contracts + wallet-core
make -C contracts/transfer wasm
make -C wallet-core wasm
make data-drivers-js           # JS bindings
```

### Local Dev Node

```bash
make prepare-dev               # One-time state setup
make run-dev                   # Ephemeral node
make run-dev-archive           # With archive storage
```

## Architecture

**Node modes**: Provisioner (default), Archive (`--features archive`), Prover (`--features prover`).

**Transaction flow**: Wallet → `rusk/` APIs → `node/` mempool → `consensus/` ordering → `vm/` execution → persistence.

**ZK stack**: `rusk-prover/` (proving) + `rusk/verifier` + `vm/host_queries` (verification). Keys from `rusk-profile/`.

## Elevated Care Zones

These require extra diligence. See `agents.md` for checklists.

| Zone | Paths |
|------|-------|
| Consensus | `consensus/` |
| Proof/sig verification | `rusk/src/lib/verifier.rs`, `vm/src/host_queries.rs`, `core/src/signatures/*` |
| Wire formats | `node-data/src/ledger/*`, `node-data/src/message.rs`, `node-data/src/encoding.rs` |
| Contract execution | `vm/src/execute.rs`, `vm/src/execute/*` |
| Genesis contracts | `contracts/stake/`, `contracts/transfer/`, `rusk-profile/` |
| Secrets | `wallet-core/`, `consensus.keys`, mnemonics, private keys |

## Git

**Branches**: `<package>/<description>` from `master` (e.g., `rusk/add-rpc-endpoint`). Don't push to `master` directly.

**Commits**: `<package>: Description` (e.g., `rusk: Add block query endpoint`). Use `ci`, `docs`, `chore` for cross-cutting.

## Conventions

- **`no_std`**: `contracts/*`, `core/`, `wallet-core/`, `data-drivers/` — don't add `std` imports
- **Serialization**: `rkyv`/`dusk-bytes` types are compatibility boundaries — don't reorder fields
- **Errors**: `thiserror` for libraries, `anyhow` at app boundaries
- **Logging**: `tracing` macros only, never log secrets, avoid `println!`
- **Secrets**: never log, use `zeroize` for buffers
- **Lockfiles**: OK to change via manifest updates, don't run `cargo update` unprompted
- **Circuit/keys**: Coordinate with maintainers before touching `rusk-profile/`
