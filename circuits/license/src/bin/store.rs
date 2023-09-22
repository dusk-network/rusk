// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use license_circuits::LicenseCircuit;
use std::io;
use storage::store_circuit;

pub fn main() -> Result<(), io::Error> {
    store_circuit::<LicenseCircuit>(Some(String::from("LicenseCircuit")))
}
