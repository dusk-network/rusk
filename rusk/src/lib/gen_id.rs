// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake2b_simd::Params;
use execution_core::{ContractId, CONTRACT_ID_BYTES};

/// Generate a [`ContractId`] address from:
/// - slice of bytes,
/// - nonce
/// - owner
pub fn gen_contract_id(
    bytes: impl AsRef<[u8]>,
    nonce: u64,
    owner: impl AsRef<[u8]>,
) -> ContractId {
    let mut hasher = Params::new().hash_length(CONTRACT_ID_BYTES).to_state();
    hasher.update(bytes.as_ref());
    hasher.update(&nonce.to_le_bytes()[..]);
    hasher.update(owner.as_ref());
    let hash_bytes: [u8; CONTRACT_ID_BYTES] = hasher
        .finalize()
        .as_bytes()
        .try_into()
        .expect("the hash result is exactly `CONTRACT_ID_BYTES` long");
    ContractId::from_bytes(hash_bytes)
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    use super::*;

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
            "2da8b6277789a88c7215789e227ef4dd97486db252e554805c7b874a17e07785"
        );
    }
}
