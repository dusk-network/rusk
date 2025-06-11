## Rusk Wallet Release Process

### 1. Update Version and Changelog

- Switch to a new branch named `rusk-wallet-release-<new version>`

- Update `Cargo.toml` to the new version.

- Update `CHANGELOG.md`, adding a new level 2 heading `## [<new version>] - <release date>`
under the `## [Unreleased]` heading.

- At the bottom of `CHANGELOG.md`, add a new entry under `<!-- Releases -->`, comparing the
new version with the last.

- Commit the changes with commit message 'rusk-wallet: release `<new version>`' 

- Make a new tag `rusk-wallet-<new version>` and push the tag.

### 2. Compile Wallet Binaries

- Using the Github Action [Compile CLI Wallet Binaries](https://github.com/dusk-network/rusk/actions/workflows/ruskwallet_build.yml), compile the wallet for the supported platforms, and use the new version tag as a parameter.

- The compiled binaries will be in a tar.gz archive inside a zip archive for each platform.
Download and unzip the archives to get the tar.gz archives inside.

### 3. Make Releases

- Make the release on the Github [Rusk Releases](https://github.com/dusk-network/rusk/releases) page with the wallet binaries tar.gz archives and the new version's changes listed in the changelog, like in [this release](https://github.com/dusk-network/rusk/releases/tag/rusk-wallet-0.2.0).

- Release on `crates.io`.

### 4. Prepare for Next Dev Iteration

- Update the version in `Cargo.toml` to the next dev version, which is the next patch version with
`-dev` appended. For example, the next dev version of 0.1.0 is 0.1.1-dev.

- Commit the changes with message 'rusk-wallet: prepare for next dev iteration'.

- Make a pull request using the section of the changelog under the heading of the new version as the PR comment, like in [this PR](https://github.com/dusk-network/rusk/pull/3715).
