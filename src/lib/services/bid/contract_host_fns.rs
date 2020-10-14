use crate::PUB_PARAMS;
use anyhow::Result;
use bid_circuits::CorrectnessCircuit;
use bid_contract::BidContract;
use canonical::{
    BridgeStore, ByteSink, ByteSource, Canon, Id32, InvalidEncoding, Store,
};
use dusk_blindbid::{bid::Bid, tree::BidTree};
use dusk_pki::StealthAddress;
use dusk_plonk::jubjub::AffinePoint as JubJubAffine;
use dusk_plonk::prelude::*;
use poseidon252::cipher::PoseidonCipher;

/// t_m in the specs
const MATURITY_PERIOD: u64 = 0;
/// t_b in the specs
const EXPIRATION_PERIOD: u64 = 0;
/// t_c in the specs
const COOLDOWN_PERIOD: u64 = 0;
const PAGE_SIZE: usize = 1024 * 4;

type BS = BridgeStore<Id32>;

pub fn bid(
    contract: &mut BidContract,
    commitment: JubJubAffine,
    hashed_secret: BlsScalar,
    nonce: BlsScalar,
    encrypted_data: PoseidonCipher,
    stealth_address: StealthAddress,
    block_height: u64,
    correctness_proof: Proof,
    spending_proof: Proof,
) -> Result<usize> {
    // Compute maturity & expiration periods
    let expiration = BlsScalar::from(block_height + MATURITY_PERIOD);
    let eligibility = BlsScalar::from(block_height + EXPIRATION_PERIOD);
    // Construct the Bid
    let bid = Bid {
        encrypted_data,
        nonce,
        stealth_address,
        hashed_secret,
        c: commitment,
        eligibility,
        expiration,
    };

    // Build Correctness Circuit env
    let mut circuit = CorrectnessCircuit {
        commitment: JubJubAffine::default(),
        value: BlsScalar::default(),
        blinder: BlsScalar::default(),
        trim_size: 1 << 10,
        pi_positions: vec![],
    };
    let pi = vec![PublicInput::AffinePoint(commitment, 0, 0)];
    let vk = rusk_profile::keys_for("bid-circuits")
        .get_verifier("bid_correctness")
        .unwrap();

    let vk = VerifierKey::from_bytes(&vk[..])?;

    circuit.verify_proof(
        &PUB_PARAMS,
        &vk,
        b"BidCorrectness",
        &correctness_proof,
        &pi,
    )?;
    // Add Bid to BidTree
    let idx = contract.inner_mut().push(bid)?;
    Ok(idx as usize)
}

#[no_mangle]
pub extern "C" fn transact(bytes: &mut [u8; PAGE_SIZE]) {
    // todo, handle errors here
    transaction(bytes).unwrap()
}

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BS as Store>::Error> {
    let store = BS::singleton();
    let mut source = ByteSource::new(bytes, store.clone());

    // read self.
    let mut slf: BidContract = Canon::<BS>::read(&mut source)?;
    // read transaction id
    let qid: u16 = Canon::<BS>::read(&mut source)?;
    match qid {
        // Bid fn call
        0 => {
            // read args
            let (
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_address,
                block_height,
                correctness_proof,
                spending_proof,
            ): (
                JubJubAffine,
                BlsScalar,
                BlsScalar,
                PoseidonCipher,
                StealthAddress,
                u64,
                Proof,
                Proof,
            ) = Canon::<BS>::read(&mut source)?;
            // Perform the bid
            let idx = bid(
                &mut slf,
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_address,
                block_height,
                correctness_proof,
                spending_proof,
            )
            .or_else(|_| Err(InvalidEncoding.into()))?;

            // HERE we should call DUSKContract.sendToObfuscated()
            // reading the rest of the fields (Spending proof)

            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return the index of the Bid
            Canon::<BS>::write(&idx, &mut sink)?;
            Ok(())
        }
        // Extend Bid call
        1 => unimplemented!(), //Pending to clarify specs and signature
                                // scheme.
        // Withdraw Bid
        2 => unimplemented!(), //Pending to clarify specs and signature
                                // scheme.
        _ => unreachable!(),
    }
}
