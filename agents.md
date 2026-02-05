# agents.md

Agent-specific guidance. Read `CLAUDE.md` first for repo map and commands.

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
> Rare and high-impact. Coordinate with maintainers first.
- **Verify**: `make -C rusk-prover test` + `make -C rusk test`

## Workflows

### TDD Bug Fixes

When a bug is reported, start by adding a test that reproduces it (it should fail on the current code). Once the failing test exists, have subagents propose minimal fixes, and only accept a change that makes the test pass (and keeps the rest of the suite green).

1. Reproduce → 2. Locate → 3. Read surrounding code → 4. Smallest fix → 5. Test → 6. Clippy

### New Feature
1. Find patterns → 2. Design minimal API → 3. Implement → 4. Add tests → 5. Clippy

### Contract Change
1. `make setup-compiler` → 2. Modify → 3. `make -C contracts/<name> wasm` → 4. Test
5. If ABI changed: update `core/`, `data-drivers/`, `wallet-core/`

### Frontend/SDK
```bash
cd w3sper.js && deno task test
```

## Verification

### PR Minimum (required)
```bash
cargo test -p <package> --release
make -C <dir> clippy
```

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

## Common Pitfalls

- Don't add `std` to `no_std` crates
- Don't reorder `rkyv` struct fields
- Don't use `println!` (use `tracing`)
- Don't assume test order
- Don't ignore clippy warnings
- Don't edit `Cargo.lock` directly

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

## Git

**Branches**: `<package>/<description>` from `master`

**Commits**: `<package>: Description`
