use crate::{ops, Contract};
use canonical_host::{MemStore, Transaction};
use dusk_blindbid::bid::Bid;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use poseidon252::cipher::PoseidonCipher;
use schnorr::single_key::{PublicKey, Signature};

type TransactionIndex = u16;

impl Contract<MemStore> {
    pub fn bid(
        commitment: JubJubAffine,
        hashed_secret: BlsScalar,
        nonce: BlsScalar,
        encrypted_data: PoseidonCipher,
        stealth_address: StealthAddress,
        block_height: u64,
        correctness_proof: Proof,
        spending_proof: Proof,
    ) -> Transaction<
        (
            TransactionIndex,
            JubJubAffine,
            BlsScalar,
            BlsScalar,
            PoseidonCipher,
            StealthAddress,
            u64,
            Proof,
            Proof,
        ),
        u64,
    > {
        Transaction::new((
            ops::BID,
            commitment,
            hashed_secret,
            nonce,
            encrypted_data,
            stealth_address,
            block_height,
            correctness_proof,
            spending_proof,
        ))
    }

    pub fn withdraw(
        signature: Signature,
        pub_key: PublicKey,
    ) -> Transaction<(TransactionIndex, Signature, PublicKey), bool> {
        Transaction::new((ops::WITHDRAW, signature, pub_key))
    }

    pub fn extend_bid(
        signature: Signature,
        pub_key: PublicKey,
        spending_proof: Proof,
        //Missing Note
    ) -> Transaction<(TransactionIndex, Signature, PublicKey, Proof), bool> {
        Transaction::new((ops::EXTEND_BID, signature, pub_key, spending_proof))
    }
}
