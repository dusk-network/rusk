use crate::leaf::BidLeaf;
use crate::Contract;
use canonical::Store;
use dusk_blindbid::bid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use poseidon252::cipher::PoseidonCipher;
use schnorr::single_key::{PublicKey, Signature};

/// t_m in the specs
const MATURITY_PERIOD: u64 = 0;
/// t_b in the specs
const EXPIRATION_PERIOD: u64 = 0;
/// t_c in the specs
const COOLDOWN_PERIOD: u64 = 0;

extern "C" {
    fn verify_sig(pk: &u8, sig: &u8, msg: &u8, ret_addr: &mut [u8; 32]);
    fn verify(ofs: &u8, ret_addr: &mut [u8; 32]);
}

impl<S: Store> Contract<S> {
    pub fn bid(
        &mut self,
        commitment: JubJubAffine,
        hashed_secret: BlsScalar,
        nonce: BlsScalar,
        encrypted_data: PoseidonCipher,
        stealth_address: StealthAddress,
        block_height: u64,
        correctness_proof: Proof,
        spending_proof: Proof,
    ) -> usize {
        // Verify proof of Correctness of the Bid.
        unsafe {
            let proof_bytes = correctness_proof.to_bytes();
            let mut ret_addr = [0u8; 32];
            // First position of the Proof byte blob pointer.
            verify(&proof_bytes[0], &mut ret_addr);
            match ret_addr[0] {
                1u8 => (),
                _ => panic!(),
            };
        };
        // Compute maturity & expiration periods
        let expiration = block_height + MATURITY_PERIOD;
        let eligibility = block_height + MATURITY_PERIOD + EXPIRATION_PERIOD;
        // Generate the Bid instance
        let mut bid = Bid {
            encrypted_data,
            nonce,
            stealth_address,
            hashed_secret,
            c: commitment,
            eligibility,
            expiration,
            pos: 0u64,
        };

        // Panic and stop the execution if the same one-time-key tries to
        // bid more than one time.
        let idx = match self
            .map()
            .get(PublicKey::from(bid.stealth_address.pk_r()))
        {
            // If no entries are found for this PK, add it to the map and the tree
            Ok(None) => {
                // Append Bid to the tree and obtain the position of it.
                let idx = self.tree_mut().push(BidLeaf { bid });
                // Link the One-time PK to the idx in the Map
                self.map_mut()
                    .insert(PublicKey::from(bid.stealth_address.pk_r()), idx)
                    .unwrap();
                // Since we checked on the `get` call that the value was not inside,
                // there's no need to check that this returns `Ok(None)`. So we just unwap
                // the `Result` and keep the `Option` untouched.
                idx
            }
            _ => panic!("Bid already present in the Tree!"),
        };

        // TODO: Inter-contract call
        idx
    }

    pub fn extend_bid(&mut self, sig: Signature, pk: PublicKey) -> bool {
        // Verify signature(
        unimplemented!()
    }

    pub fn withdraw(
        &mut self,
        sig: Signature,
        pk: PublicKey,
        spend_proof: Proof, /*Missing Note*/
    ) -> bool {
        unimplemented!()
    }
}
