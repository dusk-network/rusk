// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::hash::Hasher;
use rusk_abi::ContractId;

/// Generate a [`ContractId`] address from:
/// - slice of bytes,
/// - nonce
/// - owner
pub fn gen_contract_id(
    bytes: impl AsRef<[u8]>,
    nonce: u64,
    owner: impl AsRef<[u8]>,
) -> ContractId {
    let mut hasher = Hasher::new();
    hasher.update(bytes.as_ref());
    hasher.update(nonce.to_le_bytes());
    hasher.update(owner.as_ref());
    let hash_bytes = hasher.finalize();
    ContractId::from_bytes(hash_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    #[test]
    fn test_gen_contract_id() {
        let mut rng = StdRng::seed_from_u64(42);

        let mut bytes = vec![0; 1000];
        rng.fill_bytes(&mut bytes);

        let nonce = rng.next_u64();

        let mut owner = vec![0, 100];
        rng.fill_bytes(&mut owner);

        let contract_id =
            gen_contract_id(bytes.as_slice(), nonce, owner.as_slice());

        assert_eq!(
            hex::encode(contract_id.as_bytes()),
            "a138d3b9c87235dac6f62d1d30b75cffbb94601d9cbe5bd540b3e1e5842c8a7d"
        );
    }
}
