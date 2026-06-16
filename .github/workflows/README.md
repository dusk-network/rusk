# Workflows Overview

This directory contains the GitHub Actions workflows that automate various processes in our Rusk monorepo. Below is a detailed description of each workflow, its purpose, and its key components.

## Table of Contents
1. [General Notes](#general-notes)
2. [Workflow Files](#workflow-files)
3. [Conventions](#conventions)
4. [Adding or Modifying Workflows](#adding-or-modifying-workflows)
5. [Troubleshooting](#troubleshooting)
6. [Common Problems](#common-problems)

## General Notes
- These workflows handle tasks like CI, benchmarks, building binaries, and more.
- Workflows are triggered by various events, such as `push`, `pull_request`, or manually via `workflow_dispatch`.
- Prefer workflow-level `paths` / `paths-ignore` filters over separate change-detection jobs. Short gating jobs on self-hosted runners waste runner-minutes without reducing total occupancy.
- We heavily rely on self-hosted runners, available through `runs-on: core`. These runners are stateful.
- For self-hosted jobs that need private dependencies, isolate `GIT_CONFIG_GLOBAL` per job. Shared runner homes can retain stale `url.*.insteadOf` mappings.
- Heavy Rust jobs on `core` should keep the shared `sccache` wrapper enabled unless there is a concrete compatibility reason to bypass it.
- Workflows like `rusk_build.yml` and `ruskwallet_build.yml` use matrices for multi-OS and multi-feature builds. These ensure compatibility across multiple operating systems and architectures.
- Outputs like binaries and Docker images are stored as artifacts for download and reuse.

## Workflow Files
### [benchmarks.yml](./benchmarks.yml)
**Purpose**: Runs benchmarks for `rusk` and `node` components, and uploads the results as an artifact.  
**Trigger**: `workflow_dispatch`.

### [binary_copy.yml](./binary_copy.yml)
**Purpose**: Builds the `rusk` binary on `master` and copies it to a host directory on the runner.
**Trigger**: `push` to the `master` branch.

### [docker_image_build.yml](./docker_image_build.yml)
**Purpose**: Builds a Docker image and uploads it as an artifact.  
**Trigger**: `workflow_dispatch` (manual trigger).

### [profile_ci.yml](./profile_ci.yml)
**Purpose**: Generates proving keys using `make keys`.  
**Trigger**: `workflow_dispatch`.

### [rusk_build.yml](./rusk_build.yml)
**Purpose**: Compiles `rusk` binaries for multiple operating systems and architectures. Packages binaries with their corresponding version and features.  
**Trigger**: `workflow_dispatch`.

### [rusk_ci.yml](./rusk_ci.yml)
**Purpose**: Main PR CI for the Rust workspace. Runs `rustfmt` first inside the `clippy` job, plus the nightly test suite in parallel.
**Trigger**: `pull_request` and `workflow_dispatch`, excluding `w3sper.js`-only changes from PRs.

### [ruskwallet_build.yml](./ruskwallet_build.yml)
**Purpose**: Compiles `rusk-wallet` binaries for multiple OSes and architectures. Packages and uploads the artifacts.
**Trigger**: `workflow_dispatch`.

### [ruskwallet_ci.yml](./ruskwallet_ci.yml)
**Purpose**: Supplemental self-hosted `rusk-wallet` PR coverage on `core` for wallet-related changes.
**Trigger**: `pull_request` events scoped to wallet-related paths.

### [w3sperjs_ci.yml](./w3sperjs_ci.yml)
**Purpose**: PR and manual CI for `w3sper.js`, including Deno lint/format checks and an integration test against a local `rusk` node.
**Trigger**: `pull_request` and `workflow_dispatch`, scoped to `w3sper.js`, `wallet-core`, and the workflow file itself.

## Conventions
### Trigger Scope
Use `paths` or `paths-ignore` directly on the workflow trigger whenever a workflow only needs to run for a subset of the repo. This is cheaper and simpler than a separate change-detection job.

### Self-Hosted Checkout
For long-running jobs on `runs-on: core`, prefer a manual checkout into `/var/opt/build-cache/ci/...` instead of `actions/checkout`. This avoids failures caused by the host deleting `/home/docker/actions-runner/_work` during runner maintenance.

### Toolchains
Install only the Rust components and targets the workflow actually needs. Heavy Rust workflows on `core` should also emit `sccache` stats so cache hit rate and eviction pressure stay visible. Expensive or low-signal workflows should prefer `workflow_dispatch` over automatic triggers.

### Private Dependencies
Jobs that fetch private Git dependencies should set a per-job `GIT_CONFIG_GLOBAL` before configuring the GitHub token override.

## Adding or Modifying Workflows
1. Create a new `.yml` file in this directory.
2. Use a descriptive `name` for the workflow.
3. Document the workflow in this README.
4. Follow existing patterns for consistency.
5. Test the workflow thoroughly before merging.

## Troubleshooting
### General Debugging
Use the GitHub Actions logs to investigate failures. Expand the failing steps first, then add `set -x` or temporary debug commands only where the failure is ambiguous.

### Trigger Scope Issues
If a workflow did not run when expected, verify the `paths` / `paths-ignore` filters in the workflow trigger. Stale path filters are easy to miss after repo layout changes.

### Self-Hosted Action Failures
If a self-hosted job reports missing `action.yml`, `action.yaml`, or `Dockerfile` for a post-step, suspect a runner workspace issue before blaming the action itself. The historical failure mode on this host was deletion of `_work`, not an `_actions` bug.

### Matrix Build Failures
Check compatibility for the target platform or flags. Make sure the appropriate Rust targets, Node, or Deno versions are installed.

## Common Problems
- Self-hosted post-step failures can be secondary symptoms. Check earlier steps before treating missing `action.yml` / `Dockerfile` messages as the root cause.
- Runner-minute usage can grow quickly from short parallel jobs. Prefer folding very short checks into existing quality jobs rather than spawning separate self-hosted jobs.
