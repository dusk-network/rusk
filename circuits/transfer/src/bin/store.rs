// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io;
use storage::store_circuit;
use transfer_circuits::*;

pub fn main() -> Result<(), io::Error> {
    // store the transfer circuits
    store_circuit::<WithdrawFromTransparentCircuit>(Some(String::from(
        "WithdrawFromTransparentCircuit",
    )))?;
    store_circuit::<SendToContractTransparentCircuit>(Some(String::from(
        "SendToContractTransparentCircuit",
    )))?;
    store_circuit::<ExecuteCircuitOneTwo>(Some(String::from(
        "ExecuteCircuitOneTwo",
    )))?;
    store_circuit::<ExecuteCircuitTwoTwo>(Some(String::from(
        "ExecuteCircuitTwoTwo",
    )))?;
    store_circuit::<ExecuteCircuitThreeTwo>(Some(String::from(
        "ExecuteCircuitThreeTwo",
    )))?;
    store_circuit::<ExecuteCircuitFourTwo>(Some(String::from(
        "ExecuteCircuitFourTwo",
    )))?;
    Ok(())
}
