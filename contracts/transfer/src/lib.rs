// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(feature = "hosted", no_std)]
#![feature(lang_items)]

use canonical::Canon;
use canonical_derive::Canon;

#[derive(Canon, Debug)]
pub struct PublicKey([u8; 32]);

#[derive(Canon, Default, Debug)]
pub struct MerklePath([u8; 32]);

#[derive(Canon, Debug)]
pub struct MerkleRoot([u8; 32]);

#[derive(Canon, Debug)]
pub struct Nullifier([u8; 32]);

#[derive(Canon, Debug)]
pub struct JubJubPoint([u8; 32]);

#[derive(Canon, Default, Debug)]
pub struct BlsScalar([u8; 32]);

#[derive(Canon, Debug)]
pub struct Address([u8; 32]);

#[derive(Canon)]
pub struct ViewKey([u8; 64]);

#[derive(Canon)]
pub struct StealthAddress([u8; 64]);

#[derive(Canon)]
pub struct Signature([u8; 64]);

#[derive(Canon)]
pub struct Crossover([u8; 160]);

#[derive(Canon)]
pub struct Message([u8; 192]);

#[derive(Canon)]
pub struct Note([u8; 229]);

#[derive(Canon)]
pub struct MinerNote([u8; 73]);

#[derive(Canon)]
pub struct Proof([u8; 1040]);

#[derive(Canon)]
pub struct Transfer {
    name: [u8; 50],
    symbol: [u8; 10],
    decimals: u8,
    circulating_supply: u128,
    total_supply: u128,
}

impl Transfer {
    pub fn new(
        name: [u8; 50],
        symbol: [u8; 10],
        decimals: u8,
        circulating_supply: u128,
        total_supply: u128,
    ) -> Self {
        Transfer {
            name,
            symbol,
            decimals,
            circulating_supply,
            total_supply,
        }
    }
}

#[cfg(feature = "hosted")]
mod hosted {
    use super::*;

    use canonical::{BridgeStore, ByteSink, ByteSource, Store};

    const PAGE_SIZE: usize = 1024 * 4;

    type BS = BridgeStore<[u8; 8]>;

    impl Transfer {
        pub fn get_root(&self) -> BlsScalar {
            BlsScalar::default()
        }

        pub fn find_notes(&self, vk: &ViewKey, height: u64) -> Note {
            Note([0u8; 229])
        }

        pub fn find_nullifier(&self, nullifier: &Nullifier) -> bool {
            true
        }

        pub fn find_path(&self, note: &Note) -> MerklePath {
            MerklePath::default()
        }

        pub fn get_balance(&self, address: &Address) -> u64 {
            0
        }

        pub fn find_message(
            &self,
            address: &Address,
            pk: &PublicKey,
        ) -> Message {
            Message([0u8; 192])
        }

        pub fn get_messages(&self, address: &Address, vk: &ViewKey) -> Message {
            Message([0u8; 192])
        }

        pub fn verify_spending_proof(&self, c: &[u8], proof: &Proof) -> bool {
            true
        }

        pub fn verify_send_to_contract_transparent(
            &self,
            value: u64,
            proof: &Proof,
        ) -> bool {
            true
        }

        pub fn verify_send_to_contract_obfuscated(
            &self,
            proof: &Proof,
        ) -> bool {
            true
        }

        pub fn verify_withdraw_from_obfuscated(&self, proof: &Proof) -> bool {
            true
        }

        pub fn verify_withdraw_from_obfuscated_to_contract_transparent(
            &self,
            proof: &Proof,
        ) -> bool {
            true
        }

        pub fn verify_withdraw_from_obfuscated_to_contract_obfuscated(
            &self,
            proof: &Proof,
        ) -> bool {
            true
        }

        pub fn verify_execute(&self, proof: &Proof) -> bool {
            true
        }

        pub fn verify_anchor(&self, anchor: &BlsScalar) -> bool {
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

        pub fn increment_value(&self, address: &Address, value: u64) {}
        pub fn decrement_value(&self, address: &Address, value: u64) {}
        pub fn append_note(&self, note: &Note) {}
        pub fn append_nullifier(&self, nullifier: &Nullifier) {}
        pub fn add_message(
            &self,
            address: &Address,
            pk: &PublicKey,
            r: &JubJubPoint,
            message: &Message,
        ) {
        }
        pub fn delete_pk(&self, address: &Address, pk: &PublicKey) {}

        pub fn construct_transparent_note(
            &self,
            value: u64,
            r: &JubJubPoint,
            pk: &PublicKey,
        ) -> Note {
            Note([0u8; 229])
        }
        pub fn construct_obfuscated_note(
            &self,
            crossover: &Crossover,
            r: &JubJubPoint,
            pk: &PublicKey,
        ) -> Note {
            Note([0u8; 229])
        }

        pub fn mint(&mut self, amount: u128) -> bool {
            if self.circulating_supply + amount <= self.total_supply {
                self.circulating_supply += amount;
                return true;
            }

            false
        }

        pub fn call(&self, call_data: [u8; 1024]) {}
    }

    fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), <BS as Store>::Error> {
        let store = BS::singleton();
        let mut source = ByteSource::new(&bytes[..], store.clone());

        // read self
        let slf: Transfer = Canon::<BS>::read(&mut source)?;

        // read id
        let qid: u8 = Canon::<BS>::read(&mut source)?;
        match qid {
            // Name
            0 => {
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&slf.name, &mut sink)?;
                Ok(())
            }
            // Symbol
            1 => {
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&slf.symbol, &mut sink)?;
                Ok(())
            }
            // Decimals
            2 => {
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&slf.decimals, &mut sink)?;
                Ok(())
            }
            // Circulating supply
            3 => {
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&slf.circulating_supply, &mut sink)?;
                Ok(())
            }
            // Total supply
            4 => {
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&slf.total_supply, &mut sink)?;
                Ok(())
            }
            // Find note
            5 => {
                let vk: ViewKey = Canon::<BS>::read(&mut source)?;
                let height: u64 = Canon::<BS>::read(&mut source)?;

                let notes = slf.find_notes(&vk, height);
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&notes, &mut sink)?;
                Ok(())
            }
            // Find nullifier
            6 => {
                let nul: Nullifier = Canon::<BS>::read(&mut source)?;

                let found = slf.find_nullifier(&nul);
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&found, &mut sink)?;
                Ok(())
            }
            // Get path
            7 => {
                let note: Note = Canon::<BS>::read(&mut source)?;

                let path = slf.find_path(&note);
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&path, &mut sink)?;
                Ok(())
            }
            // Get root
            8 => {
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&slf.get_root(), &mut sink)?;
                Ok(())
            }
            // Get balance
            9 => {
                let a: Address = Canon::<BS>::read(&mut source)?;

                let balance = slf.get_balance(&a);
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&balance, &mut sink)?;
                Ok(())
            }
            // Find message
            10 => {
                let a: Address = Canon::<BS>::read(&mut source)?;
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;

                let m = slf.find_message(&a, &pk);
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&m, &mut sink)?;
                Ok(())
            }
            // Get messages
            11 => {
                let a: Address = Canon::<BS>::read(&mut source)?;
                let vk: ViewKey = Canon::<BS>::read(&mut source)?;

                let messages = slf.get_messages(&a, &vk);
                let mut sink = ByteSink::new(&mut bytes[..], store.clone());
                Canon::<BS>::write(&messages, &mut sink)?;
                Ok(())
            }
            // Send to contract transparent
            12 => {
                let a: Address = Canon::<BS>::read(&mut source)?;
                let value: u64 = Canon::<BS>::read(&mut source)?;
                let proof: Proof = Canon::<BS>::read(&mut source)?;

                // NOTE: according to the contract specs, we are supposed
                // to check here whether value < 2^64. However, it is a
                // u64, so it is impossible for the value to go beyond that.

                if !slf.verify_send_to_contract_transparent(value, &proof) {
                    panic!("proof verification failed");
                }

                slf.increment_value(&a, value);

                // TODO: adjust tx crossover here to be empty
                Ok(())
            }
            // Send to contract obfuscated
            13 => {
                let a: Address = Canon::<BS>::read(&mut source)?;
                let m: Message = Canon::<BS>::read(&mut source)?;
                let r: JubJubPoint = Canon::<BS>::read(&mut source)?;
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;
                let s_a: StealthAddress = Canon::<BS>::read(&mut source)?;
                let proof: Proof = Canon::<BS>::read(&mut source)?;

                if !slf.verify_send_to_contract_obfuscated(&proof) {
                    panic!("proof verification failed");
                }

                slf.add_message(&a, &pk, &r, &m);

                // TODO: add to mapping of ordered set??? check with tog
                // TODO: adjust tx crossover here to be empty
                Ok(())
            }
            // Withdraw from transparent
            14 => {
                let a: Address = Canon::<BS>::read(&mut source)?;
                let value: u64 = Canon::<BS>::read(&mut source)?;
                let note: Note = Canon::<BS>::read(&mut source)?;

                slf.decrement_value(&a, value);
                slf.append_note(&note);
                Ok(())
            }
            // Withdraw from obfuscated
            15 => {
                let a: Address = Canon::<BS>::read(&mut source)?;
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;
                let message: Message = Canon::<BS>::read(&mut source)?;
                let r_c: JubJubPoint = Canon::<BS>::read(&mut source)?;
                let pk_c: PublicKey = Canon::<BS>::read(&mut source)?;
                let note: Note = Canon::<BS>::read(&mut source)?;
                let proof: Proof = Canon::<BS>::read(&mut source)?;

                slf.delete_pk(&a, &pk);
                if !message.0.is_empty() {
                    slf.add_message(&a, &pk_c, &r_c, &message);
                }

                slf.verify_withdraw_from_obfuscated(&proof);
                Ok(())
            }
            // Withdraw from transparent to contract
            16 => {
                let withdraw_address: Address = Canon::<BS>::read(&mut source)?;
                let withdraw_value: u64 = Canon::<BS>::read(&mut source)?;
                let send_address: Address = Canon::<BS>::read(&mut source)?;
                let send_value: u64 = Canon::<BS>::read(&mut source)?;

                slf.decrement_value(&withdraw_address, withdraw_value);
                slf.increment_value(&send_address, send_value);

                Ok(())
            }
            // Withdraw from obfuscated to contract
            17 => {
                let withdraw_address: Address = Canon::<BS>::read(&mut source)?;
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;
                let message: Message = Canon::<BS>::read(&mut source)?;
                let r_c: JubJubPoint = Canon::<BS>::read(&mut source)?;
                let pk_c: PublicKey = Canon::<BS>::read(&mut source)?;
                let note: Note = Canon::<BS>::read(&mut source)?;
                let flag: u8 = Canon::<BS>::read(&mut source)?;
                let send_address: Address = Canon::<BS>::read(&mut source)?;
                let send_value: u64 = Canon::<BS>::read(&mut source)?;
                let send_message: Message = Canon::<BS>::read(&mut source)?;
                let r_s: JubJubPoint = Canon::<BS>::read(&mut source)?;
                let pk_s: PublicKey = Canon::<BS>::read(&mut source)?;
                let proof: Proof = Canon::<BS>::read(&mut source)?;

                slf.delete_pk(&withdraw_address, &pk);
                if !message.0.is_empty() {
                    slf.add_message(&withdraw_address, &pk_c, &r_c, &message);
                }

                if flag == 0 {
                    // NOTE: in the specification we are supposed to check here
                    // if send_value exceeds 2^64. However, since it is a u64,
                    // there is no way for it to exceed that amount.

                    slf.increment_value(&send_address, send_value);
                    if !slf.verify_withdraw_from_obfuscated_to_contract_transparent(
                        &proof,
                    ) {
                        panic!("proof verification failed");
                    }
                } else {
                    slf.add_message(&send_address, &pk_s, &r_s, &send_message);
                    if !slf
                        .verify_withdraw_from_obfuscated_to_contract_obfuscated(
                            &proof,
                        )
                    {
                        panic!("proof verification failed");
                    }
                }

                Ok(())
            }
            // Execute
            18 => {
                let anchor: BlsScalar = Canon::<BS>::read(&mut source)?;
                let nullifier_1: Nullifier = Canon::<BS>::read(&mut source)?;
                let nullifier_2: Nullifier = Canon::<BS>::read(&mut source)?;
                let nullifier_3: Nullifier = Canon::<BS>::read(&mut source)?;
                let nullifier_4: Nullifier = Canon::<BS>::read(&mut source)?;
                let nullifiers =
                    [nullifier_1, nullifier_2, nullifier_3, nullifier_4];
                let crossover: Crossover = Canon::<BS>::read(&mut source)?;
                let note_1: Note = Canon::<BS>::read(&mut source)?;
                let note_2: Note = Canon::<BS>::read(&mut source)?;
                let notes = [note_1, note_2];
                let gas_limit: u64 = Canon::<BS>::read(&mut source)?;
                let gas_price: u64 = Canon::<BS>::read(&mut source)?;
                let r: JubJubPoint = Canon::<BS>::read(&mut source)?;
                let return_pk: PublicKey = Canon::<BS>::read(&mut source)?;
                let proof: Proof = Canon::<BS>::read(&mut source)?;
                let call_data: [u8; 1024] = Canon::<BS>::read(&mut source)?;

                if !slf.verify_anchor(&anchor) {
                    panic!("anchor not found");
                }

                nullifiers.iter().for_each(|nullifier| {
                    if slf.find_nullifier(nullifier) {
                        panic!("existing nullifier found");
                    }

                    slf.append_nullifier(nullifier);
                });

                // if Crossover == 0 then set tx crossover to 0

                notes.iter().for_each(|note| {
                    slf.append_note(note);
                });

                if gas_price == 0 {
                    panic!("invalid gas price");
                }

                let cost = gas_limit * gas_price;

                slf.verify_execute(&proof);
                if !call_data.is_empty() {
                    slf.call(call_data);
                }

                if !crossover.0.is_empty() {
                    let note = slf
                        .construct_obfuscated_note(&crossover, &r, &return_pk);
                    slf.append_note(&note);
                }

                if cost > 0 {
                    let note =
                        slf.construct_transparent_note(cost, &r, &return_pk);
                    slf.append_note(&note);
                }

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

    fn transaction(
        bytes: &mut [u8; PAGE_SIZE],
    ) -> Result<(), <BS as Store>::Error> {
        let store = BS::singleton();
        let mut source = ByteSource::new(bytes, store.clone());

        // read self.
        let mut slf: Transfer = Canon::<BS>::read(&mut source)?;
        // read transaction id
        let qid: u8 = Canon::<BS>::read(&mut source)?;
        match qid {
            // Mint
            0 => {
                // Ensure the caller is authorized to mint
                // TODO: how?

                // Get amount and note
                let note: MinerNote = Canon::<BS>::read(&mut source)?;
                let value: u128 = Canon::<BS>::read(&mut source)?;

                if !slf.mint(value) {
                    panic!("could not mint {} coins", value);
                }

                Ok(())
            }
            _ => panic!("unknown opcode"),
        }
    }

    #[no_mangle]
    fn t(bytes: &mut [u8; PAGE_SIZE]) {
        // todo, handle errors here
        transaction(bytes).unwrap()
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

    impl Module for Transfer {
        const BYTECODE: &'static [u8] = include_bytes!("../transfer.wasm");
    }

    // queries
    type QueryIndex = u8;

    impl Transfer {
        pub fn name() -> Query<QueryIndex, &str> {
            Query::new(0)
        }

        pub fn symbol() -> Query<QueryIndex, &str> {
            Query::new(1)
        }

        pub fn decimals() -> Query<QueryIndex, u8> {
            Query::new(2)
        }

        pub fn circulating_supply() -> Query<QueryIndex, u128> {
            Query::new(3)
        }

        pub fn total_supply() -> Query<QueryIndex, u128> {
            Query::new(4)
        }

        pub fn find_note(
            vk: ViewKey,
            height: u64,
        ) -> Query<(QueryIndex, ViewKey, u64), Note> {
            Query::new((5, vk, height))
        }

        pub fn find_nullifier(
            nul: Nullifier,
        ) -> Query<(QueryIndex, Nullifier), bool> {
            Query::new((6, nul))
        }

        pub fn get_path(note: Note) -> Query<(QueryIndex, Note), MerklePath> {
            Query::new((7, note))
        }

        pub fn get_root() -> Query<QueryIndex, Merkleroot> {
            Query::new(8)
        }

        pub fn get_balance(a: Address) -> Query<(QueryIndex, Address), u128> {
            Query::new((9, a))
        }

        pub fn find_message(
            a: Address,
            pk: PublicKey,
        ) -> Query<(QueryIndex, Address, PublicKey), Message> {
            Query::new((10, a, pk))
        }

        pub fn get_messages(
            a: Address,
            vk: ViewKey,
        ) -> Query<(QueryIndex, Address, ViewKey), Message> {
            Query::new((11, a, vk))
        }

        pub fn send_to_contract_transparent(
            a: Address,
            value: u64,
            proof: Proof,
        ) -> Query<(Queryindex, Address, u64, Proof), ()> {
            Query::new((12, a, value, proof))
        }

        pub fn send_to_contract_obfuscated(
            a: Address,
            m: Message,
            s_a: StealthAddress,
            proof: Proof,
        ) -> Query<(Queryindex, Address, Message, StealthAddress, Proof), ()>
        {
            Query::new((13, a, m, s_a, proof))
        }

        pub fn withdraw_from_transparent(
            a: Address,
            value: u64,
            note: Note,
        ) -> Query<(Queryindex, Address, u64, Note), ()> {
            Query::new((14, a, value, note))
        }

        pub fn withdraw_from_obfuscated(
            pk: PublicKey,
            message: Message,
            r_c: JubJubPoint,
            pk_c: PublicKey,
            note: Note,
            proof: Proof,
        ) -> Query<
            (
                Queryindex,
                PublicKey,
                Message,
                JubJubPoint,
                PublicKey,
                Note,
                Proof,
            ),
            (),
        > {
            Query::new((15, pk, message, r_c, pk_c, note, proof))
        }

        pub fn withdraw_from_transparent_to_contract(
            withdraw_address: Address,
            withdraw_value: u64,
            spend_address: Address,
            spend_value: u64,
        ) -> Query<(Queryindex, Address, u64, Address, u64), ()> {
            Query::new((
                16,
                withdraw_address,
                withdraw_value,
                send_address,
                send_value,
            ))
        }

        pub fn withdraw_from_obfuscated_to_contract(
            withdraw_address: Address,
            pk: PublicKey,
            message: Message,
            r_c: JubJubPoint,
            pk_c: PublicKey,
            note: Note,
            flag: u8,
            send_address: Address,
            send_value: u64,
            send_message: Message,
            r_s: JubJubPoint,
            pk_s: PublicKey,
            proof: Proof,
        ) -> Query<
            (
                Queryindex,
                Address,
                PublicKey,
                Message,
                JubJubPoint,
                PublicKey,
                Note,
                u8,
                Address,
                u64,
                Message,
                JubJubPoint,
                PublicKey,
                Proof,
            ),
            (),
        > {
            Query::new((
                17,
                withdraw_address,
                pk,
                message,
                r_c,
                pk_c,
                note,
                flag,
                send_address,
                send_value,
                send_message,
                r_s,
                pk_s,
                proof,
            ))
        }

        pub fn execute(
            anchor: BlsScalar,
            nullifier_1: Nullifier,
            nullifier_2: Nullifier,
            nullifier_3: Nullifier,
            nullifier_4: Nullifier,
            crossover: Crossover,
            note_1: Note,
            note_2: Note,
            gas_limit: u64,
            gas_price: u64,
            r: JubJubPoint,
            return_pk: PublicKey,
            proof: Proof,
            call_data: [u8; 1024],
        ) -> Query<
            (
                Queryindex,
                BlsScalar,
                Nullifier,
                Nullifier,
                Nullifier,
                Nullifier,
                Crossover,
                Note,
                Note,
                u64,
                u64,
                JubJubPoint,
                PublicKey,
                Proof,
                [u8; 1024],
            ),
            (),
        > {
            Query::new((
                18,
                anchor,
                nullifier_1,
                nullifier_2,
                nullifier_3,
                nullifier_4,
                crossover,
                note_1,
                note_2,
                gas_limit,
                gas_price,
                r,
                return_pk,
                proof,
                call_data,
            ))
        }
    }

    // transactions
    type TransactionIndex = u8;

    impl Transfer {
        pub fn mint(
            note: MinerNote,
            value: u128,
        ) -> Transaction<(TransactionIndex, MinerNote, u128), ()> {
            Transaction::new((0, note, value))
        }
    }
}
