// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::G2Affine;

use super::*;

fn bls_public_key(key: &[u8; 96]) -> Result<BlsPublicKey, Error> {
    BlsPublicKey::from_bytes(key).map_err(|_| Error::InvalidPublicKey)
}
// check if authority update is working
#[test]
fn check_update_authority() {
    let mut contract = GovernanceContract::default();

    assert_eq!(contract.authority, BlsPublicKey::default());
    let key = G2Affine::generator().to_bytes();

    contract
        .update_authority(bls_public_key(&key).unwrap())
        .unwrap();

    assert_eq!(contract.authority, bls_public_key(&key).unwrap());
}
// check if broker update is working
#[test]
fn check_update_broker() {
    let mut contract = GovernanceContract::default();

    assert_eq!(contract.broker, BlsPublicKey::default());
    let key = G2Affine::generator().to_bytes();

    contract
        .update_broker(bls_public_key(&key).unwrap())
        .unwrap();

    assert_eq!(contract.broker, bls_public_key(&key).unwrap());
}
