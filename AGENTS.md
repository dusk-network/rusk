# Rusk — AI Agent Instructions

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
make -C <dir> test
cargo fmt --all
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

## Philosophy

1. **No area is off-limits** — some just need more care
2. **Understand before modifying** — read code, tests, trace calls
3. **Verify proportionally** — sensitive code = thorough testing
4. **Ask when uncertain** — ambiguity warrants a question
5. **Flag sensitive changes** — tell the user when touching elevated care zones
6. **Keep diffs small** — easier to review and revert
7. **Never leak secrets**

## Elevated Care Zones

Work on these with extra diligence.

### Consensus (`consensus/`)

- Understand the safety invariant before changing
- **Verify**: `make -C consensus test` + `make -C consensus testbed` + `make -C node test`
- **Watch**: fork choice, voting, timing, quorum

### Proof/Signature Verification (`verifier.rs`, `host_queries.rs`, `signatures/*`)

- Trace the verification flow
- **Verify**: `make -C vm test` + `make -C rusk test`
- **Watch**: accepting invalid or rejecting valid proofs/sigs

### Wire Formats (`node-data/src/ledger/*`, `message.rs`, `encoding.rs`)

- Check if type crosses network/storage boundaries
- **Verify**: `make -C node-data test` + `make -C node test` + `make -C rusk test`
- **Watch**: field reordering, type changes, removed fields

### Contract Execution (`vm/src/execute*`)

- Understand host function exposure
- **Verify**: `make -C vm test` + contract tests
- **Watch**: gas metering, host behavior, state access

### Genesis Contracts (`contracts/stake/`, `contracts/transfer/`)

- Understand ABI and wallet/SDK interactions
- **Verify**: `make -C contracts/<name> wasm` + `make -C contracts/<name> test`
- **Watch**: on-chain state interpretation, breaking callers

### Secrets (`wallet-core/`, consensus keys)

- Identify sensitive data flow
- **Verify**: `make -C wallet-core test` + review for logging
- **Watch**: logging secrets, missing zeroization

### Circuit/Prover Keys (`rusk-profile/`, `rusk-prover/`)
>
> Rare and high-impact. Coordinate with maintainers first.

- **Verify**: `make -C rusk-prover test` + `make -C rusk test`

### Submodules (`contracts/`)

Do not change the `contracts/` submodule pointer unless explicitly instructed. Moving the pointer changes which version of the genesis contracts rusk builds and tests against — this must be a deliberate decision.

## Workflows

### TDD Bug Fixes

When a bug is reported, start by adding a test that reproduces it (it should fail on the current code). Once the failing test exists, have subagents propose minimal fixes, and only accept a change that makes the test pass (and keeps the rest of the suite green).

1. Reproduce → 2. Locate → 3. Read surrounding code → 4. Smallest fix → 5. Test → 6. `cargo fmt --all` → 7. Clippy

### New Feature

1. Find patterns → 2. Design minimal API → 3. Implement → 4. Add tests → 5. `cargo fmt --all` → 6. Clippy

### Contract Change

1. `make setup-compiler` → 2. Modify → 3. `make -C contracts/<name> wasm` → 4. Test
5. If ABI changed: update `core/`, `data-drivers/`, `wallet-core/`

### Frontend/SDK

```bash
cd w3sper.js && deno task test
```

## Verification

See [PR Minimum](#pr-minimum) in Commands.

### Expand When

- Package is widely depended on (`core/`, `node-data/`) → test dependents
- Elevated care zone → follow zone-specific verification
- Multi-crate → `make clippy`, consider `make test`

## Decision Guidelines

### Do Without Asking

- Localized bug fixes
- Test improvements
- Doc/comment fixes in files you're modifying
- CI/tooling fixes
- Frontend changes with passing checks
- Non-genesis test contracts
- Lockfile changes from manifest updates

### Ask First

- Ambiguous requirements
- Architectural decisions
- Multi-subsystem impact (3+ crates, Rust/JS boundary)
- Compatibility concerns
- Performance trade-offs in hot paths
- ABI/encoding changes
- Adding deps to core crates

### When to Stop

If you can't understand the invariant, structure, or what would break — ask rather than guess.

## Integration Points

| Component | Verify With |
|-----------|-------------|
| `core/` | `make -C core test` + `make -C vm test` |
| `node-data/` | `make -C node-data test` + `make -C node test` + `make -C rusk test` |
| `vm/` | `make -C vm test` + `make -C rusk test` |
| `contracts/` | `make -C contracts test` + `make prepare-dev` |
| `consensus/` | `make -C consensus testbed` |
| `rusk/` | `make -C rusk test` |
| `wallet-core/` | `make -C wallet-core test` + `make -C rusk-wallet test` |
| Frontends | `npm run checks` |
| `w3sper.js/` | `deno task test` |

## Error Recovery

```bash
# Contract build fails
make setup-compiler
rustup target add wasm32-unknown-unknown

# State init fails
make prepare-dev

# Build fails
cargo clean -p <crate> && cargo build -p <crate> --release
```

## Conventions

- **`no_std`**: `contracts/*`, `core/`, `wallet-core/`, `data-drivers/` — don't add `std` imports
- **Serialization**: `rkyv`/`dusk-bytes` types are compatibility boundaries — don't reorder fields
- **Errors**: `thiserror` for libraries, `anyhow` at app boundaries
- **Logging**: `tracing` macros only, never log secrets, avoid `println!`
- **Secrets**: never log, use `zeroize` for buffers
- **Lockfiles**: OK to change via manifest updates, don't run `cargo update` unprompted
- **Circuit/keys**: Coordinate with maintainers before touching `rusk-profile/`
- **Clippy**: don't ignore or suppress warnings — fix the underlying issue
- **Test order**: don't assume tests run in a specific order

## Git

**Branches**: `<package>/<description>` from `master` (e.g., `rusk/add-rpc-endpoint`). Don't push to `master` directly.

### Commit messages

Format: `<scope>: <Description>` — imperative mood, capitalize first word after colon.

**One commit per crate per concern.** Each commit touches exactly one crate and one logical concern. Never bundle changes to different crates in one commit, and don't mix unrelated changes within the same crate either (e.g. a dependency API adaptation and a new feature are separate commits even if both touch `vm/`). Order commits bottom-up through the dependency chain (e.g. `core` → `vm` → `rusk`).

Canonical scopes — exactly one prefix per crate:

| Scope | Directory |
|-------|-----------|
| `core` | `core/` |
| `node-data` | `node-data/` |
| `node` | `node/` |
| `consensus` | `consensus/` |
| `vm` | `vm/` |
| `rusk` | `rusk/` (the main binary crate) |
| `rusk-wallet` | `rusk-wallet/` |
| `rusk-recovery` | `rusk-recovery/` |
| `rusk-prover` | `rusk-prover/` |
| `rusk-profile` | `rusk-profile/` |
| `rusk-test` | `rusk-test/` |
| `wallet-core` | `wallet-core/` |
| `data-driver` | `data-drivers/` |
| `w3sper` | `w3sper.js/` |
| `contracts` | `contracts/` submodule pointer updates |

Cross-cutting (not crate-scoped):

| Scope | When |
|-------|------|
| `workspace` | Root `Cargo.toml`, cross-crate dependency bumps, Makefile recipes |
| `ci` | `.github/workflows/` |
| `docs` | Documentation-only changes |
| `chore` | Housekeeping (submodule URLs, repo splits, etc.) |
| `docker` | Docker files |

Examples:
- `core: Add sha256 ABI host query`
- `vm: Add withdrawal replay call hook`
- `rusk: Gate new host queries behind Boreas activation`
- `workspace: Update dusk dependencies`

Do not:
- Bundle changes to multiple crates in one commit
- Use `WIP` or `fixup` commits (squash before push)
- Use generic messages like `fix typo` or `update code` without context
