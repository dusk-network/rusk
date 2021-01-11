// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::leaf::BidLeaf;
use crate::Contract;
use canonical::Store;
use dusk_blindbid::bid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use phoenix_core::Note;
use poseidon252::cipher::PoseidonCipher;
use schnorr::single_key::{PublicKey, Signature};

/// TODO: Still waiting for values from the research side.
/// t_m in the specs
const MATURITY_PERIOD: u64 = 0;
/// t_b in the specs
const EXPIRATION_PERIOD: u64 = 10;
/// t_c in the specs
const COOLDOWN_PERIOD: u64 = 0;

extern "C" {
    fn verify_schnorr_sig(pk: &u8, sig: &u8, msg: &u8) -> i32;
    fn verify_proof(
        pub_inputs_len: usize,
        pub_inputs: &u8,
        proof: &u8,
        verif_key: &u8,
    ) -> i32;
}

impl<S: Store> Contract<S> {
    pub fn bid(
        &mut self,
        commitment: JubJubAffine,
        hashed_secret: BlsScalar,
        nonce: BlsScalar,
        encrypted_data: PoseidonCipher,
        stealth_address: StealthAddress,
        // This will be avaliable inside of the contract scope.
        block_height: u64,
        correctness_proof: Proof,
        _spending_proof: Proof,
        pub_inputs: [[u8; 33]; 1],
    ) -> (bool, usize) {
        // Setup error flag to false
        let mut err_flag = false;
        // Verify proof of Correctness of the Bid.
        // TODO: Mask this unsafe somewhere else.
        unsafe {
            // TODO: We should avoid that.
            let proof_bytes = correctness_proof.to_bytes();
            match verify_proof(
                1usize,
                &pub_inputs[0][0],
                &proof_bytes[0],
                &crate::BID_CORRECTNESS_VK[0],
            ) {
                1i32 => (),
                0i32 => err_flag = true,
                // TODO: CHECK the panic! impl since it panics.
                _ => panic!("Parameter got malformed"),
            };
        };

        if err_flag {
            return (err_flag, usize::MAX);
        }

        // Compute maturity & expiration periods
        let expiration = block_height + MATURITY_PERIOD + EXPIRATION_PERIOD;
        let eligibility = block_height + MATURITY_PERIOD;
        // Generate the Bid instance
        let bid = Bid {
            encrypted_data,
            nonce,
            stealth_address,
            hashed_secret,
            c: commitment,
            eligibility,
            expiration,
            pos: u64::MAX,
        };
        // Panic and stop the execution if the same one-time-key tries to
        // bid more than one time.
        let idx = match self
            // TODO: Rename since it's confusing.
            .map()
            .get(PublicKey::from(bid.stealth_address.pk_r()))
        {
            // If no entries are found for this PK, add it to the map and the
            // tree
            Ok(None) => {
                // Append Bid to the tree and obtain the position of it.
                // TODO: Add an issue in Poseidon for the size obtention in the
                // internal push impl.
                let idx = self.tree_mut().push(BidLeaf { bid });
                // Link the One-time PK to the idx in the Map
                // Since we checked on the `get` call that the value was not
                // inside, there's no need to check that this
                // returns `Ok(None)`. So we just unwrap
                // the `Result` and keep the `Option` untouched.
                self.map_mut()
                    .insert(PublicKey::from(bid.stealth_address.pk_r()), idx)
                    .unwrap();
                idx
            }
            _ => {
                err_flag = true;
                // Return whatever
                usize::MAX
            }
        };

        // TODO: Inter-contract call
        (err_flag, idx)
    }

    pub fn extend_bid(&mut self, sig: Signature, pk: PublicKey) -> bool {
        // Setup error flag to false
        let mut err_flag = false;
        // Check wether there's an entry on the map for the pk.
        let idx = match self.map().get(pk) {
            // If no entries are found for this PK it's just an err since there
            // are no bids related to this PK to be extended.
            Ok(None) => {
                err_flag = true;
                usize::MAX
            }
            Ok(Some(idx)) => idx as usize,
            Err(_) => {
                err_flag = true;
                usize::MAX
            }
        };

        // In case there was an error, we simply return
        if err_flag && idx == usize::MAX {
            return err_flag;
        }

        // Verify the signature by getting `t_e` from the Bid and calling the
        // VERIFY_SIG host fn.
        // Fetch the bid object from the tree getting a &mut to it.
        let tree = self.tree_mut();
        let mut bid = *tree.get_mut(idx as u64).expect("TODO");
        let msg_bytes = BlsScalar::from(bid.bid.expiration.clone()).to_bytes();
        let pk_bytes = pk.to_bytes();
        let sig_bytes = sig.to_bytes();

        // Verify schnorr sig.
        let res = unsafe {
            verify_schnorr_sig(&pk_bytes[0], &sig_bytes[0], &msg_bytes[0])
        };

        if res == 0i32 {
            err_flag = true;
            return err_flag;
        }

        // Assuming now that the result of the verification is true, we now
        // should update the expiration of the Bid by `EXPIRATION_PERIOD`.
        bid.bid.expiration += EXPIRATION_PERIOD;
        err_flag
    }

    pub fn withdraw(
        &mut self,
        sig: Signature,
        pk: PublicKey,
        _note: Note,
        _spend_proof: Proof,
        block_height: u64,
    ) -> bool {
        // Setup error flag to false
        let mut err_flag = false;
        // Check wether there's an entry on the map for the pk.
        let idx = match self.map().get(pk) {
            // If no entries are found for this PK it's just an err since there
            // are no bids related to this PK to be extended.
            Ok(None) => {
                err_flag = true;
                usize::MAX
            }
            Ok(Some(idx)) => idx as usize,
            Err(_) => {
                err_flag = true;
                usize::MAX
            }
        };

        // In case there was an error, we simply return
        if err_flag && idx == usize::MAX {
            return err_flag;
        }

        // Fetch bid info from the tree. Note that we can safely unwrap here due
        // to the checks done previously while getting the idx from the map.
        let bid = self
            .tree()
            .get(idx as u64)
            .expect("Unexpected error. Map & Tree are out of sync.");

        if bid.bid.expiration < (block_height + COOLDOWN_PERIOD) {
            // If we arrived here, the bid is elegible of withdraw
            // Now we need to check wether the signature is correct.
            let msg_bytes = BlsScalar::from(bid.bid.expiration).to_bytes();
            let pk_bytes = pk.to_bytes();
            let sig_bytes = sig.to_bytes();
            // Verify schnorr sig.
            if unsafe {
                verify_schnorr_sig(&pk_bytes[0], &sig_bytes[0], &msg_bytes[0])
            } == 1i32
            {
                // Inter contract call

                // If the inter-contract call succeeds, we need to clean the
                // tree & map. Note that if we clean the entry
                // corresponding to this `PublicKey` from the
                // map there will be no need to do so from the tree. Since the
                // rest of the functions rely on the map to gain
                // access to the bid that is inside of the tree.
                self.map_mut()
                    .remove(pk)
                    .expect("Canon Store error happened.");
                // TODO: Zeroize in the tree
                return err_flag;
            } else {
                err_flag = true;
                return err_flag;
            }
        } else {
            err_flag = true;
            return err_flag;
        }
    }
}
