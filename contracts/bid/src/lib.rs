// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(feature = "hosted", no_std)]
#![feature(lang_items)]

use canonical::Canon;
use canonical_derive::Canon;

const ELIGIBILITY_PERIOD: u64 = 250000;

#[derive(Canon, Debug)]
pub struct PublicKey([u8; 32]);

#[derive(Canon)]
pub struct Signature([u8; 64]);

#[derive(Canon)]
pub struct Note([u8; 192]);

#[derive(Canon)]
pub struct Bid {
    pub encrypted_data: [u8; 96],
    pub nonce: [u8; 32],
    pub stealth_address: [u8; 64],
    pub hashed_secret: [u8; 32],
    pub c: [u8; 32],
    pub eligibility: [u8; 8],
    pub expiration: [u8; 8],
}

#[derive(Canon)]
pub struct Proof([u8; 1040]);

#[derive(Canon, Debug)]
pub struct BidContract;

impl BidContract {
    pub fn new() -> Self {
        BidContract {}
    }
}

#[cfg(feature = "hosted")]
mod hosted {
    use super::*;

    use canonical::{BridgeStore, ByteSink, ByteSource, Store};

    const PAGE_SIZE: usize = 1024 * 4;

    type BS = BridgeStore<[u8; 8]>;

    impl BidContract {
        pub fn fetch_bid(&self, pk: &PublicKey) -> Bid {
            Bid {
                encrypted_data: [0u8; 96],
                nonce: [0u8; 32],
                stealth_address: [0u8; 64],
                hashed_secret: [0u8; 32],
                c: [0u8; 32],
                eligibility: [0u8; 8],
                expiration: [0u8; 8],
            }
        }

        pub fn verify_bid_correctness(&self, c: &[u8], proof: &Proof) -> bool {
            true
        }

        pub fn write_bid(&self, bid: &Bid) -> bool {
            true
        }

        pub fn verify_ed25519_signature(
            &self,
            pk: &PublicKey,
            sig: &Signature,
            msg: &u8,
            msg_len: u64,
        ) -> bool {
            true
        }
    }

    fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), <BS as Store>::Error> {
        let store = BS::singleton();
        let mut source = ByteSource::new(&bytes[..], store.clone());

        // read self
        let slf: BidContract = Canon::<BS>::read(&mut source)?;

        // read id
        let qid: u8 = Canon::<BS>::read(&mut source)?;
        match qid {
            // Find bid
            0 => {
                // Read out a pk
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;
                let bid = slf.fetch_bid(&pk);
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&bid, &mut sink)?;
                Ok(())
            }
            // Bid
            1 => {
                // Read out a bid
                let bid: Bid = Canon::<BS>::read(&mut source)?;
                // Read out bid correctness proof
                let correctness_proof: Proof = Canon::<BS>::read(&mut source)?;
                // Read out spending proof
                let spending_proof: Proof = Canon::<BS>::read(&mut source)?;

                // Ensure correctness of bid
                if !slf.verify_bid_correctness(&bid.c, &correctness_proof) {
                    panic!("proof verification for bid correctness failed");
                }

                // Write bid to bid tree
                if !slf.write_bid(&bid) {
                    panic!("could not write bid to the tree");
                }

                // Inter-contract call to send to contract obfuscated
                // TODO: how do we do this?

                Ok(())
            }
            // Extend bid
            2 => {
                // Read out a pk
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;
                // Read out a sig
                let sig: Signature = Canon::<BS>::read(&mut source)?;
                // Fetch Bid pending extension
                let mut bid: Bid = Canon::<BS>::read(&mut source)?;

                // Verify sig
                if !slf.verify_ed25519_signature(
                    &pk,
                    &sig,
                    &bid.encrypted_data[0],
                    320,
                ) {
                    panic!("invalid signature");
                }

                // Update bid timestamp
                let mut expiration = u64::from_le_bytes(bid.expiration);
                expiration += ELIGIBILITY_PERIOD;
                bid.expiration = expiration.to_le_bytes();

                // Write the bid back to the tree
                if !slf.write_bid(&bid) {
                    panic!("could not write bid to the tree");
                }

                Ok(())
            }
            // Withdraw bid
            3 => {
                // Read out a pk
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;
                // Read out a sig
                let sig: Signature = Canon::<BS>::read(&mut source)?;
                // Read out a note
                let note: Note = Canon::<BS>::read(&mut source)?;
                // Read out a spending proof
                let proof: Proof = Canon::<BS>::read(&mut source)?;

                // Check that the bid has expired
                let bid = slf.fetch_bid(&pk);
                // if bid.expiration > hf.block_height {
                //     panic!("bid hasn't expired yet");
                // }

                // Verify signature
                if !slf.verify_ed25519_signature(
                    &pk,
                    &sig,
                    &bid.expiration[0],
                    8,
                ) {
                    panic!("invalid signature");
                }

                // Inter-contract call to withdraw from obfuscated
                // TODO: how do we do this?
                Ok(())
            }
            _ => panic!("unknown opcode"),
        }
    }

    #[no_mangle]
    fn q(bytes: &mut [u8; PAGE_SIZE]) {
        // todo, handle errors here
        let _ = query(bytes);
    }

    mod panic_handling {
        use core::panic::PanicInfo;

        #[panic_handler]
        fn panic(_: &PanicInfo) -> ! {
            loop {}
        }

        #[lang = "eh_personality"]
        extern "C" fn eh_personality() {}
    }
}

#[cfg(feature = "host")]
mod host {
    use super::*;
    use canonical_host::{Module, Query};

    impl Module for BidContract {
        const BYTECODE: &'static [u8] = include_bytes!("../bidcontract.wasm");
    }

    // queries
    type QueryIndex = u8;

    impl BidContract {
        pub fn find_bid(pk: PublicKey) -> Query<(QueryIndex, PublicKey), Bid> {
            Query::new((0, pk))
        }

        pub fn bid(
            bid: Bid,
            correctness_proof: Proof,
            spending_proof: Proof,
        ) -> Query<(QueryIndex, Bid, Proof, Proof), ()> {
            Query::new((1, bid, correctness_proof, spending_proof))
        }

        pub fn extend_bid(
            pk: PublicKey,
            sig: Signature,
        ) -> Query<(QueryIndex, PublicKey, Signature), ()> {
            Query::new((2, pk, sig))
        }

        pub fn withdraw_bid(
            pk: PublicKey,
            sig: Signature,
            note: Note,
            spending_proof: Proof,
        ) -> Query<(QueryIndex, PublicKey, Signature, Note, Proof), ()>
        {
            Query::new((3, pk, sig, note, spending_proof))
        }
    }
}
