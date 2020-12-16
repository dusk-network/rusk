use crate::{ops, Contract};
use canonical::{BridgeStore, ByteSink, ByteSource, Canon, Id32, Store};
use dusk_blindbid::bid::Bid;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use poseidon252::cipher::PoseidonCipher;
use schnorr::single_key::{PublicKey, Signature};

const PAGE_SIZE: usize = 1024 * 4;

type BS = BridgeStore<Id32>;
type QueryIndex = u16;
type TransactionIndex = u16;

fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), <BS as Store>::Error> {
    let store = BS::default();
    let mut source = ByteSource::new(&bytes[..], store.clone());

    // read self.
    let slf: Contract<BS> = Canon::<BS>::read(&mut source)?;

    // read query id
    let qid: QueryIndex = Canon::<BS>::read(&mut source)?;
    match qid {
        ops::FIND_BID => {
            /*
            // Read idx
            let idx: u64 = Canon::<BS>::read(&mut source)?;
            // Get the leaf
            let ret = slf.get_leaf(idx);
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            Canon::<BS>::write(&ret, &mut sink)?;
            */
            unimplemented!()
        }
        _ => panic!(""),
    }
}

#[no_mangle]
fn q(bytes: &mut [u8; PAGE_SIZE]) {
    let _ = query(bytes);
}

fn transaction(
    bytes: &mut [u8; PAGE_SIZE],
) -> Result<(), <BS as Store>::Error> {
    let store = BS::default();
    let mut source = ByteSource::new(bytes, store.clone());

    // read self.
    let mut slf: Contract<BS> = Canon::<BS>::read(&mut source)?;
    // read transaction id
    let qid: TransactionIndex = Canon::<BS>::read(&mut source)?;
    match qid {
        ops::BID => {
            // Read host-sent args
            let commitment: JubJubAffine = Canon::<BS>::read(&mut source)?;
            let nonce: BlsScalar = Canon::<BS>::read(&mut source)?;
            let stealth_address: StealthAddress =
                Canon::<BS>::read(&mut source)?;
            let encrypted_data: PoseidonCipher =
                Canon::<BS>::read(&mut source)?;
            let hashed_secret: BlsScalar = Canon::<BS>::read(&mut source)?;
            let block_height: u64 = Canon::<BS>::read(&mut source)?;
            // Fat pointer to the Proof objects.
            let correctness_proof: Proof = Canon::<BS>::read(&mut source)?;
            let spending_proof: Proof = Canon::<BS>::read(&mut source)?;
            // Call bid contract fn
            let idx = slf.bid(
                commitment,
                hashed_secret,
                nonce,
                encrypted_data,
                stealth_address,
                block_height,
                correctness_proof,
                spending_proof,
            );
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&(idx as u64), &mut sink)
        }
        ops::WITHDRAW => {
            // Read host-sent args
            let sig: Signature = Canon::<BS>::read(&mut source)?;
            let pk: PublicKey = Canon::<BS>::read(&mut source)?;
            let spending_proof: Proof = Canon::<BS>::read(&mut source)?;
            let exec_res = slf.withdraw(sig, pk, spending_proof);
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // Return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // Return result
            Canon::<BS>::write(&exec_res, &mut sink)
        }
        ops::EXTEND_BID => {
            // Read host-sent args
            let sig: Signature = Canon::<BS>::read(&mut source)?;
            let pk: PublicKey = Canon::<BS>::read(&mut source)?;
            let exec_res = slf.extend_bid(sig, pk);
            let mut sink = ByteSink::new(&mut bytes[..], store.clone());
            // return new state
            Canon::<BS>::write(&slf, &mut sink)?;
            // return result
            Canon::<BS>::write(&exec_res, &mut sink)
        }
        _ => panic!(""),
    }
}

#[no_mangle]
fn t(bytes: &mut [u8; PAGE_SIZE]) {
    transaction(bytes).unwrap()
}
