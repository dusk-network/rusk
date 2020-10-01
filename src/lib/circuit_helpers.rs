// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use anyhow::Result;
use dusk_plonk::prelude::*;
use std::fs::File;
use std::io::prelude::*;

/// CRS path.
const PUB_PARAMS_FILE: &'static str = "pub_params_dev.bin";
/// BlindBid Circuit ProverKey path.
const BLINDBID_CIRCUIT_PK_PATH: &'static str = "blindbid_circ.pk";
/// BlindBid Circuit VerifierKey path.
const BLINDBID_CIRCUIT_VK_PATH: &'static str = "blindbid_circ.vk";

/// Read PublicParameters from the binary file they're stored on.
pub fn read_pub_params() -> Result<PublicParameters> {
    let mut pub_params_file = File::open(PUB_PARAMS_FILE)?;
    let mut buff = vec![];
    pub_params_file.read_to_end(&mut buff)?;
    let result: PublicParameters = bincode::deserialize(&buff)?;
    Ok(result)
}

pub fn read_blindcid_circuit_pk() -> Result<ProverKey> {
    let mut pub_params_file = File::open(BLINDBID_CIRCUIT_PK_PATH)?;
    let mut buff = vec![];
    pub_params_file.read_to_end(&mut buff)?;
    let result: ProverKey = ProverKey::from_bytes(&buff)?;
    Ok(result)
}

pub fn read_blindcid_circuit_vk() -> Result<VerifierKey> {
    let mut pub_params_file = File::open(BLINDBID_CIRCUIT_VK_PATH)?;
    let mut buff = vec![];
    pub_params_file.read_to_end(&mut buff)?;
    let result: VerifierKey = VerifierKey::from_bytes(&buff)?;
    Ok(result)
}
