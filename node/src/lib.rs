// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::user::provisioners::Provisioners;
#[derive(Debug)]
pub enum Error {}

/// Reads config parameters
pub fn read_config() -> Result<(), Error> {
    Ok(())
}

///  Sets up a node and execute lifecycle loop.
pub fn bootstrap() -> Result<(), Error> {
    let empty_list = Provisioners::new();
    println!("Hello provisioners {:?}", empty_list);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
}
