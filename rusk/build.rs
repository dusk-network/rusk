// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Buildfile for the rusk crate.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure we run the build script again even if we change just the build.rs
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../Cargo.lock");

    // Get crate version + commit + toolchain for `-v` arg support.
    println!(
        "cargo:rustc-env=GIT_HASH={}",
        rustc_tools_util::get_commit_hash().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=COMMIT_DATE={}",
        rustc_tools_util::get_commit_date().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=RUSTC_RELEASE_CHANNEL={}",
        rustc_tools_util::get_channel().unwrap_or_default()
    );

    Ok(())
}
