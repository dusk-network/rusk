// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;
use include_dir::{include_dir, Dir, DirEntry};
use lazy_static::lazy_static;

const PROV_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/provisioners/");

lazy_static! {
    pub static ref PROVISIONERS: [PublicKey; PROV_DIR.entries().len()] = {
        let files = PROV_DIR.entries().iter();
        let keys: Vec<PublicKey> = files.map(parse_file).collect();
        keys.try_into().unwrap()
    };
}

fn parse_file(dir_entry: &DirEntry) -> PublicKey {
    let filename = dir_entry.path().file_name().unwrap();
    let file = PROV_DIR.get_file(filename).unwrap();
    let b58_bytes = file.contents();
    parse_key(b58_bytes)
}

fn parse_key(bs58: &[u8]) -> PublicKey {
    let bytes = bs58::decode(bs58).into_vec().expect("Base58 decoding");
    // FIXME: This is only done because `BadLength` is not implemented for
    //  `dusk_bls12_381_sign::Error`. Otherwise we could use
    //  `PublicKey::from_slice`
    assert_eq!(bytes.len(), PublicKey::SIZE);
    let mut key_bytes = [0u8; PublicKey::SIZE];
    key_bytes.copy_from_slice(&bytes[..]);

    PublicKey::from_bytes(&key_bytes)
        .expect("Genesis consensus key to be valid")
}
