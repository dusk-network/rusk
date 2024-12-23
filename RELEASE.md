# Rusk Monorepo Release Process

## **1. Branching Strategy**

### `master` Branch
- **Purpose:** Active development.
- **Policy:**
  - All new features behind feature flags.
  - Merged only after passing CI, integration tests, and approval.

### `devnet` Branch
- **Purpose:** Weekly snapshots from `master` for early testing.
- **Policy:** Updated weekly, deploying directly to the Devnet environment.

### Release Branches (`rusk-release-vX.Y`)
- **Purpose:** Stable releases for mainnet or testnet deployments.
- **Why We Need It:**
  - **Hotfix Management:** Allows hotfixes to be applied without branching from tags.
  - **Long-Term Support:** Enables long-term support and backporting of fixes.
  - **Clear Release Tracking:** Provides a clear path for version progression.

- **Creation:** Every 1-3 months or as needed.
- **Hotfixes:** Created directly on the (affected) release branch.

---

## **2. Tagging and Release Strategy**

### Libraries
- `dusk-abi-vX.Y.Z` (rusk-framework release?)
- `dusk-core-vX.Y.Z` (rusk-framework release?)
- `dusk-wallet-core-vX.Y.Z` (rusk-framework release?)
- `w3sper.js-vX.Y.Z`

### User-Facing Apps
- `explorer-vX.Y.Z`
- `rusk-wallet-vX.Y.Z`
- `web-wallet-vX.Y.Z`

### Blockchain Node
- `rusk-vX.Y.Z` (node binary release)
- `rusk-framework-vX.Y.Z` (library releases for components such as ABI, core, wallet-core)

**Example:** `rusk-v1.2.3`

---

## **3. Release Lifecycle**

1. **Day 0:** Create release branch `rusk-release-vX.Y` from `master`.
2. **Day 1:** Deploy release to **Testnet**.
3. **Day 7:** Release validated and stabilized in Testnet.
4. **Day 10:** Finalize integration tests and prepare for mainnet deployment.
5. **Day 14:** Community announcement and gradual provisioners upgrade.
6. **Day 16:** Deploy `rusk-vX.Y.Z` to **Mainnet**.

---

## **4. Compatibility and Testing**

### **Testing Phases**
1. **Unit Tests:** Every PR must pass.
2. **Integration Tests:** Wallet, SDKs, contracts, circuits.
3. **Network Tests:** Simulate large-scale node clusters.
4. **Compatibility Tests:** Ensure backward compatibility with old node versions.

---

## **5. Release Artifacts**

- **Crates.io Packages:** For Rust libraries (`dusk-abi`, `dusk-core`, `dusk-wallet-core`, `rusk-wallet`).
- **NPM/JSR Packages:** For JavaScript libraries (`w3sper.js`).
- **Docker Images:** Published for node: default and archive feature.
- **GitHub Releases:** For `rusk-wallet`, `web-wallet`, `explorer`, `rusk`, and `rusk-framework`.
- **Svelte Builds:** Uploaded to production servers after integration testing.

---

## **6. Release Documentation**

- **GitHub Release Notes:** Each release includes:
  - Summary of changes.
  - Migration steps (if applicable).
  - Framework upgrade details.
- **Changelog Management:** Use automated changelog generation from PR labels (`breaking-change`, `enhancement`, `bug-fix`).

---

## **7. Environment Versioning Policy**

- **Local Development:** `X.Y.Z-dev`
- **Devnet:** `X.Y.Z-rc`
- **Testnet:** `X.Y.Z-beta`
- **Mainnet:** `X.Y.Z`
