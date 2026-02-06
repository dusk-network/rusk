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
- Reusable actions and patterns, such as `dorny/paths-filter` for change detection and `actions/checkout`, are heavily utilized to standardize processes.
- We heavily rely on self-hosted runners, available through `runs-on: core`. These runners are stateful. It's only recommended to use other runners if you need stronger consistenty guarantees, as they're slower.
- Workflows like `rusk_build.yml` and `ruskwallet_build.yml` use matrices for multi-OS and multi-feature builds. Thesse ensure compatibility across multiple operating systems and architectures.
- Outputs like binaries and Docker images are stored as artifacts for download and reuse.

## Workflow Files
### [benchmarks.yml](./binary_copy.yml)
**Purpose**: Runs benchmarks for `rusk` and `node` components, and uploads the results as an artifact.  
**Trigger**: `push` to the `master` branch.

### [binary_copy.yml](./binary_copy.yml)
**Purpose**: Builds and copies the `rusk` binary to a host directory on the runner.  
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
**Purpose**: CI for the `rusk` repository. Executes formatting, linting, and tests.  
**Trigger**: `pull_request` events.

### [ruskwallet_build.yml](./ruskwallet_build.yml)
**Purpose**: Compiles `rusk-wallet` binaries for multiple OSes and architectures. Packages and uploads the artifacts  
**Trigger**: `workflow_dispatch`.

### [ruskwallet_ci.yml](./ruskwallet_ci.yml)
**Purpose**: CI for the `rusk-wallet` repository, with specific nightly tests for multiple platforms.  
**Trigger**: `pull_request` events.

### [w3sperjs_ci.yml](./w3sperjs_ci.yml)
**Purpose**: CI for `w3sper.js`, executing linting and test tasks.  
**Trigger**: `pull_request` and `workflow_dispatch`.

### [webwallet_ci.yml](./webwallet_ci.yml)
**Purpose**: CI for the `web-wallet`, executing lints, tests, typechecks and builds the app.  
**Trigger**: `pull_request` events.

## Adding or Modifying Workflows
1. Create a new `.yml` file in this directory.
2. Use a descriptive `name` for the workflow.
3. Document the workflow in this README.
4. Follow existing patterns for consistency.
5. Test the workflow thoroughly before merging.

## Troubleshooting
### General Debugging
Use the GitHub Actions logs to investigate failures. Checking the jobs and collapsing the runs often provide a lot of output information on versions or filters used. Add `set -x` or debug specific commands to problematic steps to gather more information.

### Change Detection Issues
Verify the path patterns in `dorny/paths-filter`. Make sure the `filters` section includes all relevant paths. You can check the `changes` job in a workflow run and check the `dorny/paths-filter` run.

### Matrix Build Failures
Check compatibility for the target platform or flags. Make sure the appropriate Rust targets, Node or Deno versions are installed. For Rust, you can check the `dsherret/rust-toolchain-file` run in a workflow run for the installer release.

## Common Problems
It often occurs that CI reports `action.yml`/`action.yaml`/`Dockerfile` are not found. This is often a false-positive where the post-run fails due to a prior failure. Investigate the workflow run by check if the earlier steps report any other issue.
