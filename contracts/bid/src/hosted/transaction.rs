// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::fake_abi;
use crate::{contract_constants::*, leaf::BidLeaf, Contract};
use alloc::vec::Vec;
use canonical::Store;
use core::ops::DerefMut;
use dusk_blindbid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::{Ownable, PublicKey};
use phoenix_core::Note;
use schnorr::Signature;

impl<S: Store> Contract<S> {
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
        mut bid: Bid,
        correctness_proof: Vec<u8>,
        _spending_proof: Vec<u8>,
    ) -> bool {
        // Setup sucess var to true
        let mut success = true;

        // Verify proof of Correctness of the Bid.
        if !fake_abi::verify_proof(
            correctness_proof,
            crate::BID_CORRECTNESS_VK.to_vec(),
            b"bid-correctness".to_vec(),
            PublicInput::AffinePoint(bid.commitment(), 0, 0)
                .to_bytes()
                .to_vec(),
        ) {
            return false;
        }

        // Obtain the current block_height.
        let block_height = dusk_abi::block_height();
        // Compute maturity & expiration periods
        let expiration = block_height + MATURITY_PERIOD + EXPIRATION_PERIOD;
        let eligibility = block_height + MATURITY_PERIOD;

        // Mutate the Bid and add the correct timestamps.
        bid.set_eligibility(eligibility);
        // FIXME: This should not be needed. We should have a better API in blindbid.
        //  Since the API for blindbid was decided to be set as `extend_expiration` instead of
        // `set_expiration`. We now are forced to ensure that the expiration is 0 here and then, we
        // sum `expiration` to it.
        assert!(bid.expiration() == 0u64);
        bid.extend_expiration(expiration);

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
            let idx = self.tree_mut().push(BidLeaf(bid));
            // Link the One-time PK to the idx in the Map
            // Since we checked on the `get` call that the value was not
            // inside, there's no need to check that this
            // returns `Ok(None)`. So we just unwrap
            // the `Result` and keep the `Option` untouched.
            self.key_idx_map_mut()
                .insert(*bid.stealth_address().pk_r(), idx)
                .unwrap();
        } else {
            success = false;
        };

        // TODO: Inter-contract call
        success
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
        let tree = self.tree_mut();
        let mut branch_mut = tree
            .get_mut(idx as u64)
            .expect("No leaf was attached to the provided idx");
        let bid: &mut BidLeaf = branch_mut.deref_mut();

        // Verify schnorr sig.
        if !fake_abi::verify_schnorr_sig(
            pk,
            sig,
            BlsScalar::from(bid.0.expiration()),
        ) {
            return false;
        }

        // Assuming now that the result of the verification is true, we now
        // should update the expiration of the Bid by `EXPIRATION_PERIOD`.
        bid.0.extend_expiration(EXPIRATION_PERIOD);
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
        _note: Note,
        _spend_proof: Vec<u8>,
        block_height: u64,
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
        let bid = self
            .tree()
            .get(idx as u64)
            .expect("Unexpected error. Map & Tree are out of sync.");

        if bid.0.expiration() < (block_height + COOLDOWN_PERIOD) {
            // If we arrived here, the bid is elegible for withdrawal.
            // Now we need to check wether the signature is correct.
            // Verify schnorr sig.
            if !fake_abi::verify_schnorr_sig(
                pk,
                sig,
                BlsScalar::from(bid.0.expiration()),
            ) {
                return false;
            };
            // Inter contract call

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

// TODO: Until PLONK is no_std compatible and we can serialize PublicInputs
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubScalar;

const BLS_SCALAR: u8 = 1;
const JUBJUB_SCALAR: u8 = 2;
const JUBJUB_AFFINE: u8 = 3;

/// Public Input
#[derive(Debug, Copy, Clone)]
enum PublicInput {
    /// Scalar Input
    BlsScalar(BlsScalar, usize),
    /// Embedded Scalar Input
    JubJubScalar(JubJubScalar, usize),
    /// Point as Public Input
    AffinePoint(JubJubAffine, usize, usize),
}

impl PublicInput {
    /// Returns the serialized-size of the `PublicInput` structure.
    const fn serialized_size() -> usize {
        33usize
    }

    /// Returns the byte-representation of a [`PublicInput`].
    /// Note that the underlying variants of this enum have different
    /// sizes on it's byte-representation. Therefore, we need to return
    /// the biggest one to set it as the default one.
    fn to_bytes(&self) -> [u8; Self::serialized_size()] {
        let mut bytes = [0u8; Self::serialized_size()];
        match self {
            Self::BlsScalar(scalar, _) => {
                bytes[0] = BLS_SCALAR;
                bytes[1..33].copy_from_slice(&scalar.to_bytes());
                bytes
            }
            Self::JubJubScalar(scalar, _) => {
                bytes[0] = JUBJUB_SCALAR;
                bytes[1..33].copy_from_slice(&scalar.to_bytes());
                bytes
            }
            Self::AffinePoint(point, _, _) => {
                bytes[0] = JUBJUB_AFFINE;
                bytes[1..Self::serialized_size()]
                    .copy_from_slice(&point.to_bytes());
                bytes
            }
        }
    }
}
