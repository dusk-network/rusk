// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{contract_constants::*, leaf::BidLeaf, leaf::Expiration, Contract};
use alloc::vec::Vec;
use core::ops::DerefMut;
use dusk_abi::Transaction;
use dusk_blindbid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_pki::{Ownable, PublicKey, StealthAddress};
use dusk_schnorr::Signature;
use microkelvin::Nth;
use phoenix_core::{Message, Note};
use transfer_contract::Call;

impl Contract {
    /// This function allows to the contract caller to setup a Bid related to a
    /// one-time identity of his/her property that will allow the user to
    /// participate in the bidding process of the consensus as well as to
    /// prove that is part of the bidders set.
    ///
    /// This function will first of all, verify that the Bid is correct by
    /// verifying the BidCorrectness Proof.
    /// Then it will include the Bid into the PoseidonTree of the contract and
    /// link the One-Time identity of the user to the index that the bid
    /// occupies in the tree. Finally it will execute an inter-contract call
    /// sending the `spending_proof` and a `note` to the transfer contract.
    /// Which will execute the transaction of Dusk to the contract account.
    pub fn bid(
        &mut self,
        message: Message,
        hashed_secret: BlsScalar,
        stealth_address: StealthAddress,
        correctness_proof: Vec<u8>,
        spending_proof: Vec<u8>,
    ) -> bool {
        // Setup sucess var to true
        let mut success = true;

        // Verify proof of Correctness of the Bid.
        if !rusk_abi::verify_proof(
            correctness_proof,
            crate::BID_CORRECTNESS_VD.to_vec(),
            alloc::vec![(message.value_commitment()).into()],
        ) {
            return false;
        }

        // Obtain the current block_height.
        let block_height = dusk_abi::block_height();
        // Compute maturity & expiration periods
        let expiration = block_height + MATURITY_PERIOD + VALIDITY_PERIOD;
        let eligibility = block_height
            + MATURITY_PERIOD
            + VALIDITY_PERIOD
            + (EPOCH - (block_height % EPOCH));

        // Generate the bid
        let mut bid = Bid::new(
            message,
            hashed_secret,
            stealth_address,
            eligibility,
            expiration,
        );

        // Panic and stop the execution if the same one-time-key tries to
        // bid more than one time.
        if self
            .key_idx_map()
            // If no entries are found for this PK, add it to the map and the
            // tree
            .get(*bid.stealth_address().pk_r())
            .unwrap()
            .is_none()
        {
            // Append Bid to the tree and obtain the position of it.
            if let Ok(idx) = self
                .tree_mut()
                .push(BidLeaf::new(bid, Expiration(expiration)))
            {
                // Link the One-time PK to the idx in the Map
                // Since we checked on the `get` call that the value was not
                // inside, there's no need to check that this
                // returns `Ok(None)`. So we just unwrap
                // the `Result` and keep the `Option` untouched.
                self.key_idx_map_mut()
                    .insert(*bid.stealth_address().pk_r(), idx as usize)
                    .unwrap();
            }
        } else {
            return false;
        };

        // Inter-contract call to lock bidder's funds in the Bid contract.
        let call = Call::send_to_contract_obfuscated(
            dusk_abi::callee(),
            message,
            stealth_address,
            spending_proof,
        );

        let call = Transaction::from_canon(&call);
        dusk_abi::transact_raw(self, &rusk_abi::transfer_contract(), &call)
            .expect("Failed to send dusk to Bid contract");

        true
    }

    // TODO: Check wether we allow to extend long-time expired Bids.
    // see: https://github.com/dusk-network/rusk/issues/163

    /// This function allows to the contract caller to extend the expiration
    /// time for his/her `Bid`. That means, remain longer in the Bidding
    /// consensus process with the same `Bid` and therefore the same
    /// One-time identity.
    pub fn extend_bid(&mut self, sig: Signature, pk: PublicKey) -> bool {
        // Setup success to true
        let mut success = true;
        // Check wether there's an entry on the map for the pk.
        let idx = match self.key_idx_map().get(pk) {
            // If no entries are found for this PK it's just an err since there
            // are no bids related to this PK to be extended.
            Ok(None) => {
                success = false;
                usize::MAX
            }
            Ok(Some(idx)) => *idx as usize,
            Err(_) => {
                success = false;
                usize::MAX
            }
        };

        // In case there was an error, we simply return
        if !success && idx == usize::MAX {
            return false;
        }

        // Verify the signature by getting `t_e` from the Bid and calling the
        // VERIFY_SIG host fn.
        // Fetch the bid object from the tree getting a &mut to it.
        let mut branch_mut = if let Ok(Some(branch)) =
            self.tree_mut().as_mut().nth_mut(idx as u64)
        {
            branch
        } else {
            return false;
        };
        let bid: &mut BidLeaf = branch_mut.deref_mut();

        // Check wether the maturity and expiration periods are within bounds.
        let block_height = dusk_abi::block_height();
        let bid_expiration = bid.bid().expiration().clone();
        // σh + Δmaturity​ ≥ B.Bhexpiration​)
        assert!(block_height + MATURITY_PERIOD >= bid_expiration);
        // B.Bhexpiration​ > σh
        assert!(bid_expiration > block_height);

        // Verify schnorr sig.
        if !rusk_abi::verify_schnorr_sign(
            sig,
            pk,
            BlsScalar::from(bid_expiration),
        ) {
            return false;
        }

        // Assuming now that the result of the verification is true, we now
        // should update the expiration of the Bid by `VALIDITY_PERIOD`.
        bid.bid_mut().extend_expiration(VALIDITY_PERIOD);
        bid.expiration_mut().0 += VALIDITY_PERIOD;
        success
    }

    /// This function allows to the contract caller to withdraw it's `Bid` and
    /// therefore the funds placed to place it in the contract.
    ///
    /// Note that to be able to withdraw a `Bid`, it needs to reach a certain
    /// time which corresponds to the `expiration` time of the bid plus the
    /// `COOLDOWN_PERIOD`.
    ///
    /// Once this execution suceeds, any links between the bidder, as well as
    /// it's one-time identity and the Bid itself will be erased from the
    /// contract storage which will return some gas to the caller.
    pub fn withdraw(
        &mut self,
        sig: Signature,
        pk: PublicKey,
        note: Note,
        spend_proof: Vec<u8>,
    ) -> bool {
        // Setup success to true
        let mut success = true;
        // Check wether there's an entry on the map for the pk.
        let idx = match self.key_idx_map().get(pk) {
            // If no entries are found for this PK it's just an err since there
            // are no bids related to this PK to be extended.
            Ok(None) => {
                success = false;
                usize::MAX
            }
            Ok(Some(idx)) => *idx as usize,
            Err(_) => {
                success = false;
                usize::MAX
            }
        };

        // In case there was an error, we simply return
        if !success && idx == usize::MAX {
            return false;
        }

        // Fetch bid info from the tree. Note that we can safely unwrap here due
        // to the checks done previously while getting the idx from the map.
        let bid = if let Ok(Some(bid)) = self.tree().get(idx as u64) {
            bid
        } else {
            return false;
        };

        if *bid.bid().expiration() < dusk_abi::block_height() {
            // If we arrived here, the bid is elegible for withdrawal.
            // Now we need to check wether the signature is correct.
            // Verify schnorr sig.
            if !rusk_abi::verify_schnorr_sign(
                sig,
                pk,
                BlsScalar::from(*bid.bid().expiration()),
            ) {
                return false;
            };

            // Withdraw from Obfuscated call to retire the funds of the bidder.
            let call = Call::withdraw_from_obfuscated(
                *bid.bid().message(),
                *bid.bid().stealth_address(),
                note,
                note.value_commitment().into(),
                spend_proof,
            );

            let call = Transaction::from_canon(&call);
            dusk_abi::transact_raw(self, &rusk_abi::transfer_contract(), &call)
                .expect("Failed to withdraw dusk from the Bid contract");

            // If the inter-contract call succeeds, we need to clean the
            // tree & map. Note that if we clean the entry
            // corresponding to this `PublicKey` from the
            // map there will be no need to do so from the tree. Since the
            // rest of the functions rely on the map to gain
            // access to the bid that is inside of the tree.
            self.key_idx_map_mut()
                .remove(pk)
                .expect("Canon Store error happened.");
            // TODO: Zeroize in the tree the leaf that corresponds to the idx
            // linked to `pk` in the map.
            // See: https://github.com/dusk-network/rusk/issues/164
            true
        } else {
            false
        }
    }
}
