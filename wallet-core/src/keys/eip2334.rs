// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This module defines the EIP2334 Deterministic Account Hierarchy path for BLS
//! keys as defined at <https://eips.ethereum.org/EIPS/eip-2334>

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};

use crate::keys::eip2333;

/// The base derivation path for Dusk EIP-2333 BLS12-381 derivation.
/// `m / purpose / coin_type /  account / use`
///
/// `purpose` is set to 12381 as per EIP-2334, which is the name of the curve.
/// `coin_type` type is set to 744. The number that Dusk uses.
/// `account` is incremented when the user wants to create a new account i.e.,
/// get a new address.
/// `use` is **always** set to 0 for moonlight. It is set to 1 for staking keys
/// to separate them from moonlight keys.
pub const EIP_2334_BASE_PATH: &str = "m/12381/744/0/0";

/// Converts a given index nummber to the corresponding derivation path of
/// moonlight EIP-2333 BLS12-381 derivation.
pub(crate) fn index_to_path(index: usize) -> String {
    let index_str = index.to_string();

    // put the index at the correct position (account)
    // m/12381/744/index/0
    let mut path_parts: Vec<&str> = EIP_2334_BASE_PATH.split('/').collect();
    path_parts[3] = &index_str;

    path_parts.join("/")
}

/// Generates a [`BlsSecretKey`] from the master secret key and index.
///
/// The key is generated through EIP-2333.
///
/// When generating a new key pair, the [`derive_bls_key_pair`] function is
/// preferred to be used, as it generates both the secret and public key at
/// once.
///
/// # Panics
///
/// This function panics when invariants are violated, which should never
/// happen.
#[must_use]
pub fn derive_bls_sk(master_sk: &BlsSecretKey, index: usize) -> BlsSecretKey {
    let path = index_to_path(index);

    eip2333::derive_bls_sk(master_sk, &path).expect("Should always succeed")
}

/// Generates the [`BlsSecretKey`] & [`BlsPublicKey`] pair from the given master
/// secret key and index.
///
/// The key is generated through EIP-2333.
///
/// # Panics
///
/// This function panics when invariants are violated, which should never
/// happen.
#[must_use]
pub fn derive_bls_key_pair(
    master_sk: &BlsSecretKey,
    index: usize,
) -> (BlsSecretKey, BlsPublicKey) {
    let path = index_to_path(index);

    let sk = eip2333::derive_bls_sk(master_sk, &path)
        .expect("Should always succeed");
    let pk = BlsPublicKey::from(&sk);

    (sk, pk)
}

/// Generates a [`BlsPublicKey`] from the given [`BlsSecretKey`]
///
/// When generating a new key pair, the [`derive_bls_key_pair`] function is
/// preferred to be used, as it generates both the secret and public key at
/// once.
#[must_use]
pub fn derive_bls_pk(sk: &BlsSecretKey) -> BlsPublicKey {
    BlsPublicKey::from(sk)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test path to index conversion
    #[test]
    fn test_index_to_path_conversion() {
        let path = "m/12381/744/0/0";

        let indexes = index_to_path(0);
        assert_eq!(indexes, path);

        let path = "m/12381/744/1/0";

        let indexes = index_to_path(1);
        assert_eq!(indexes, path);

        let path = "m/12381/744/150/0";

        let indexes = index_to_path(150);
        assert_eq!(indexes, path);
    }
}
