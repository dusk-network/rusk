// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::DeserializableSlice;
use once_cell::sync::Lazy;

static PROVISIONERS: Lazy<[PublicKey; 5]> = Lazy::new(|| {
    [
        parse_key(include_bytes!("../assets/provisioners/node_0.cpk")),
        parse_key(include_bytes!("../assets/provisioners/node_1.cpk")),
        parse_key(include_bytes!("../assets/provisioners/node_2.cpk")),
        parse_key(include_bytes!("../assets/provisioners/node_3.cpk")),
        parse_key(include_bytes!("../assets/provisioners/node_4.cpk")),
    ]
});

static TESTNET_PROVISIONERS: Lazy<[PublicKey; 5]> = Lazy::new(|| {
    [
        parse_key(include_bytes!("../assets/provisioners/testnet/node_0.cpk")),
        parse_key(include_bytes!("../assets/provisioners/testnet/node_1.cpk")),
        parse_key(include_bytes!("../assets/provisioners/testnet/node_2.cpk")),
        parse_key(include_bytes!("../assets/provisioners/testnet/node_3.cpk")),
        parse_key(include_bytes!("../assets/provisioners/testnet/node_4.cpk")),
    ]
});

pub fn keys(testnet: bool) -> &'static [PublicKey; 5] {
    match testnet {
        true => &TESTNET_PROVISIONERS,
        false => &PROVISIONERS,
    }
}

pub static DUSK_KEY: Lazy<PublicKey> =
    Lazy::new(|| parse_key(include_bytes!("../assets/dusk.cpk")));

fn parse_key(key_bytes: &[u8]) -> PublicKey {
    PublicKey::from_slice(key_bytes).expect("Genesis consensus key to be valid")
}
