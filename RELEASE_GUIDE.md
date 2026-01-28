# Release Guide

This guide documents the release process for the internal Rust crates in this
repository, including the final `dusk-rusk` release step.

## Prerequisites

- `cargo-release` installed:
  - `cargo install cargo-release`
- `cargo-set-version` installed (via cargo-edit):
  - `cargo install cargo-edit`

Notes:

- `cargo release`:
  - creates a commit with the updated CHANGELOG and Cargo.toml to create the release.
  - commits the changes and creates the correct tag.
  - does not publish the crate on crates.io
  - does a dry run unless you pass `--execute`.
- `cargo set-version`
  - bumps the `Cargo.toml` version field to the specified version.
  - example: `cargo set-version -p dusk-rusk --bump alpha` will bump `dusk-rusk`
    to the next alpha version.
  - does not commit the changes

## Determine Which Crates Need a Release

1) Identify the last release/merge reference in the commit history.
2) List changed crates since the last release tag or merge:
   - `git diff <last-tag>..HEAD --name-only`
   - Map paths to crate directories to decide which of the crates changed.
3) Any crates that are dependencies of the `dusk-rusk` crate, whose public
   API or published artifacts changed, even if the changes were indirect
   (dependency updates), will need a new release.

## Remove Path Dependencies For Untouched Crates

The workspace uses path dependencies for internal crates in `Cargo.toml` under
`[workspace.dependencies]`. When releasing a subset of crates, temporarily
remove path dependencies for untouched crates so their versions resolve from
crates.io instead of local paths.

In `Cargo.toml`, for crates you are *not* releasing, comment out the
`path = "./<crate>/"` form and enable the plain version line.

For crates that are part of the current release train, leave the `path` entries
as is.

Example where `dusk-consensus` is *not* part of the release train:

```diff
[workspace.dependencies]
-# dusk-consensus = "1.4.0"
-dusk-consensus = { version = "1.4.1-alpha.1", path = "./consensus/" }
+dusk-consensus = "1.4.0"
+# dusk-consensus = { version = "1.4.1-alpha.1", path = "./consensus/" }
```

## Publish Each Crate

Respecting dependency paths, the crates need to be release in a specific order:

Tier 0 (no internal deps)

- `dusk-data-driver`
- `dusk-core`
- `rusk-profile`

Tier 1 (depends only on Tier 0)

- `dusk-transfer-contract-dd` (deps: `dusk-core`, `dusk-data-driver`)
- `dusk-stake-contract-dd` (deps: `dusk-core`, `dusk-data-driver`)
- `dusk-vm` (deps: `dusk-core`)
- `rusk-prover` (deps: `dusk-core`, `rusk-profile`)
- `node-data` (deps: `dusk-core`)
- `wallet-core` (deps: `dusk-core`)

Tier 2

- `rusk-recovery` (deps: `rusk-profile`; optional: `dusk-core`, `dusk-vm`)
- `dusk-consensus` (deps: `node-data`, `dusk-core`)

Tier 3

- `node` (deps: `dusk-consensus`, `node-data`, `dusk-core`; dev-deps: `wallet-core`)

Tier 4

- `dusk-rusk` (deps: `dusk-transfer-contract-dd`, `dusk-stake-contract-dd`, `dusk-data-driver`, `dusk-core`, `dusk-vm`, `rusk-profile`; optional: `rusk-prover`, `node`, `dusk-consensus`, `node-data`, `rusk-recovery`; dev-deps: `wallet-core`)

For each crate that needs a release do the following:

### 1. Update Crate Changelogs

Make sure all changes of the release are tracked in the crate's `CHANGELOG`.
Also track the updates of dependencies of crates in the tiers higher than 0.

### 2. Release Crate

Release the crate using `cargo release`.

Example for a patch release of `dusk-core`:

```sh
cargo release patch -p dusk-core
```

This will run a dry-run of the command. Make sure the suggested changes are
correct and *only then* execute it by adding `--execute`.

This will:

- In the `Cargo.toml`:
  - Update the crate version.
- In the `CHANGELOG`:
  - Move relevant entries from “Unreleased” into a new version section.
  - Create a new empty `Unreleased` section.
  - Update the version links.
- Commit the changes.
- Create a version tag for the release.

### 3. Publish Crate on `crates.io`

Publish the crate on crates.io using the `cargo publish` command:

```sh
cargo publish -p [crate-name]
```

Note that this step can be omitted for `dusk-rusk` itself.

### 4. Prepare Crate for Next Development Iteration

After releasing a crate, bump it to the next development pre-release version.

Example:

```sh
cargo set-version -p [crate-name] --bump alpha
```

This bumps to the next alpha pre-release in the crate's own `Cargo.toml`:

```diff
- version = "1.4.2"
+ version = "1.4.3-alpha.1"
```

As well as commenting out the path dependency in the workspace `Cargo.toml`:

```diff
- # node = { version = "1.4.1", package = "dusk-node" }
- node = { version = "1.4.2", path = "./node/", package = "dusk-node" }
+ node = { version = "1.4.2", package = "dusk-node" }
+ # node = { version = "1.4.3-alpha.1", path = "./node/", package = "dusk-node" }
```

## Restore Workspace Path Dependencies

After the release of `dusk-rusk` releases are done, restore the internal
workspace dependencies back to `path = "./<crate>/"` in `Cargo.toml` so local
development uses workspace paths.

## Release `dusk-rusk` on Gighub

- Create a release PR for `dusk-rusk` that lists all the changes of the release
  (use the `CHANGELOG` entries).
- Generate the release artifacts.
- Create a github release with the same `CHANGELOG` entries and link the
  generated artifacts (you might need to use a linux machine for that)

## Change Internal Dependencies to Path

If everything went well, all of the uncommented internal dependencies in the
workspace `Cargo.toml` should point to the latest release version on crate.io,
and all the internal dependencies, that use the path, should be commented out.
To prepare the workspace for the next development iteration we need to swap that
around: uncommenting the internal crates with path dependencies and commenting
the ones that use the latest version on crates.io.
