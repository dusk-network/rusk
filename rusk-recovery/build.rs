// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use cargo_toml::{Dependency, DependencyDetail, Manifest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure we run the build script again even if we change just the build.rs
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../Cargo.lock");

    // Set RUSK_KEY_PLONK_VERSION env variable
    let plonk_version = parse_plonk_version();
    println!("cargo:rustc-env=RUSK_KEY_PLONK_VERSION={plonk_version}",);

    Ok(())
}

/// Returns that string that defines the plonk-version
///
/// First, it tries to find the plonk version in the current crate's Cargo.toml.
/// If it doesn't find it, it tries to find it in the workspace's Cargo.toml.
/// If it doesn't find it there either, it panics.
fn parse_plonk_version() -> String {
    let cargo_toml = include_bytes!("./Cargo.toml");
    let cargo_toml = Manifest::from_slice(cargo_toml)
        .expect("Couldn't parse workspace manifest");

    let plonk_dep = &cargo_toml.dependencies["dusk-plonk"];

    let mut version = match plonk_dep {
        Dependency::Simple(v) => v.clone(),
        Dependency::Detailed(DependencyDetail {
            version: Some(v), ..
        }) => v.clone(),
        _ => {
            // Dependency not found in the current crate, try to find it in the
            // workspace
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR is not set");

            let parent_toml = std::path::Path::new(&manifest_dir)
                .parent()
                .expect("parent folder to exists in dev")
                .join("Cargo.toml");
            // Read parent_toml bytes
            let cargo_toml =
                std::fs::read(parent_toml).expect("Cargo.toml to be read");

            let cargo_toml = Manifest::from_slice(&cargo_toml)
                .expect("Couldn't parse workspace manifest");

            let plonk_dep = &cargo_toml
                .workspace
                .expect("Cargo.toml at crate root should define a workspace")
                .dependencies["dusk-plonk"];
            match plonk_dep {
                Dependency::Simple(v) => v.clone(),
                Dependency::Detailed(DependencyDetail {
                    version: Some(v),
                    ..
                }) => v.clone(),
                _ => {
                    panic!("Couldn't find plonk version",)
                }
            }
        }
    };
    // sanitize plonk version
    if version.starts_with('=') {
        version.remove(0);
    }
    version
}
