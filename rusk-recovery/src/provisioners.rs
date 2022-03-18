// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::DeserializableSlice;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PROVISIONERS: [PublicKey; 15] = [
        parse_key(include_bytes!("../provisioners/node_0.cpk")),
        parse_key(include_bytes!("../provisioners/node_1.cpk")),
        parse_key(include_bytes!("../provisioners/node_2.cpk")),
        parse_key(include_bytes!("../provisioners/node_3.cpk")),
        parse_key(include_bytes!("../provisioners/node_4.cpk")),
        parse_key(include_bytes!("../provisioners/node_5.cpk")),
        parse_key(include_bytes!("../provisioners/node_6.cpk")),
        parse_key(include_bytes!("../provisioners/node_7.cpk")),
        parse_key(include_bytes!("../provisioners/node_8.cpk")),
        parse_key(include_bytes!("../provisioners/node_9.cpk")),
        parse_key(include_bytes!("../provisioners/node_10.cpk")),
        parse_key(include_bytes!("../provisioners/node_11.cpk")),
        parse_key(include_bytes!("../provisioners/node_12.cpk")),
        parse_key(include_bytes!("../provisioners/node_13.cpk")),
        parse_key(include_bytes!("../provisioners/node_14.cpk")),
    ];
}

fn parse_key(key_bytes: &[u8]) -> PublicKey {
    PublicKey::from_slice(key_bytes).expect("Genesis consensus key to be valid")
}
