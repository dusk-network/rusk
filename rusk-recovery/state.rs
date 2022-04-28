// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod genesis;
pub mod provisioners;
mod ziputil;

use std::{env, error::Error, fs, io::Write};

use crate::state::provisioners::PROVISIONERS;
use http_req::request;
const STATE_URL: &str =
    "https://dusk-infra.ams3.digitaloceanspaces.com/keys/rusk-state.zip";

pub fn embed_state() {
    let state = get_state().unwrap();
    println!("{}", state.len());

    //write the state in folder specified by OUT_DIR env var
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = format!("{}/state.zip", out_dir);
    let mut file = fs::File::create(out_path).unwrap();
    file.write_all(&state).unwrap();
}

/// return the bytes of the state depending on RUSK_BUILD_STATE env
/// If it's set, it build the state from scratch. Otherwise, it download the
/// state from the network
fn get_state() -> Result<Vec<u8>, Box<dyn Error>> {
    if env::var("RUSK_BUILD_STATE").unwrap_or("".to_string()) == "true" {
        genesis::build_state()
    } else {
        download_state()
    }
}

/// Downloads the state into the rusk profile directory.
fn download_state() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buffer = vec![];
    let response = request::get(STATE_URL, &mut buffer)?;

    // only accept success codes.
    if !response.status_code().is_success() {
        return Err(format!(
            "State download error: HTTP {}",
            response.status_code()
        )
        .into());
    }
    Ok(buffer)
}
