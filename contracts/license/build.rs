// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Buildfile for the license contract, to set the necessary environment
/// variables.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = rusk_profile::get_rusk_keys_dir()?;

    println!("Keys dir is {keys_dir:?}");
    // Ensure we run the build script again even if we change just the build.rs
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../Cargo.lock");

    // Set RUSK_BUILT_KEYS_PATH for `.vd` resolver
    println!(
        "cargo:rustc-env=RUSK_BUILT_KEYS_PATH={}",
        keys_dir.to_str().unwrap()
    );

    let circuit_profile = rusk_profile::Circuit::from_name("LicenseCircuit")?;

    // Set ID_LICENSE_CIRCUIT
    println!(
        "cargo:rustc-env=ID_{}={}",
        circuit_profile.name().to_uppercase(),
        circuit_profile.id_str()
    );

    Ok(())
}
