// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PROVISIONERS: [PublicKey; 1] =
        [parse_key(include_bytes!("../provisioners/dusk.0.cpk")),];
}

fn parse_key(bytes: &[u8]) -> PublicKey {
    // FIXME: This is only done because `BadLength` is not implemented for
    //  `dusk_bls12_381_sign::Error`. Otherwise we could use
    //  `PublicKey::from_slice`
    assert_eq!(bytes.len(), PublicKey::SIZE);

    let mut key_bytes = [0u8; PublicKey::SIZE];
    key_bytes.copy_from_slice(bytes);

    PublicKey::from_bytes(&key_bytes)
        .expect("Genesis consensus key to be valid")
}
