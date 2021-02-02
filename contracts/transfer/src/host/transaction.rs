// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ops, Contract};

use canonical::Store;
use canonical_host::{MemStore, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_plonk::proof_system::proof::Proof;
use phoenix_core::Note;

impl<S: Store> Contract<S> {
    pub fn send_to_contract_transparent(
        address: BlsScalar,
        value: u64,
        spend_proof: Proof,
    ) -> Transaction<(u8, BlsScalar, u64, Proof), bool> {
        Transaction::new((
            ops::TX_SEND_TO_CONTRACT_TRANSPARENT,
            address,
            value,
            spend_proof,
        ))
    }
}
