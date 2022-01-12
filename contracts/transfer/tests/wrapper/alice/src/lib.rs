// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use canonical_derive::Canon;
use dusk_abi::{ContractId, Transaction};
use dusk_pki::StealthAddress;
use phoenix_core::{Message, Note};

pub const TX_PING: u8 = 0x01;
pub const TX_WITHDRAW: u8 = 0x02;
pub const TX_WITHDRAW_OBFUSCATED: u8 = 0x03;
pub const TX_WITHDRAW_TO_CONTRACT: u8 = 0x04;

#[derive(Debug, Clone, Canon)]
pub struct Alice {
    transfer: ContractId,
}

impl From<ContractId> for Alice {
    fn from(transfer: ContractId) -> Self {
        Self { transfer }
    }
}

impl Alice {
    pub const fn identifier() -> &'static [u8; 5] {
        b"alice"
    }
}

#[cfg(target_arch = "wasm32")]
mod hosted {
    use super::*;

    use canonical::{Canon, CanonError, Sink, Source};
    use dusk_abi::{ContractState, ReturnValue};
    use rusk_abi::PaymentInfo;
    use transfer_contract::Call;

    const PAGE_SIZE: usize = 1024 * 4;

    impl Alice {
        pub fn ping(&mut self) {
            dusk_abi::debug!("Alice ping");
        }

        pub fn withdraw(&mut self, value: u64, note: Note, proof: Vec<u8>) {
            let call = Call::withdraw_from_transparent(value, note, proof);
            let call = Transaction::from_canon(&call);
            let transfer = self.transfer;

            dusk_abi::transact_raw(self, &transfer, &call)
                .expect("Failed to withdraw");
        }

        pub fn withdraw_obfuscated(
            &mut self,
            message: Message,
            message_address: StealthAddress,
            output: Note,
            proof: Vec<u8>,
        ) {
            let call = Call::withdraw_from_obfuscated(
                message,
                message_address,
                output,
                proof,
            );
            let call = Transaction::from_canon(&call);
            let transfer = self.transfer;

            dusk_abi::transact_raw(self, &transfer, &call)
                .expect("Failed to withdraw obfuscated!");
        }

        pub fn withdraw_to_contract(&mut self, to: ContractId, value: u64) {
            let call = Call::withdraw_from_transparent_to_contract(to, value);
            let call = Transaction::from_canon(&call);
            let transfer = self.transfer;

            dusk_abi::transact_raw(self, &transfer, &call)
                .expect("Failed to withdraw");
        }
    }

    fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
        let mut source = Source::new(&bytes[..]);

        let _contract = Alice::decode(&mut source)?;
        let qid = u8::decode(&mut source)?;

        match qid {
            rusk_abi::PAYMENT_INFO => {
                let ret = PaymentInfo::Any(None);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            _ => panic!(""),
        }
    }

    #[no_mangle]
    fn q(bytes: &mut [u8; PAGE_SIZE]) {
        let _ = query(bytes);
    }

    fn transaction(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
        let mut source = Source::new(bytes);

        let mut contract = Alice::decode(&mut source)?;
        let tid = u8::decode(&mut source)?;

        match tid {
            TX_PING => contract.ping(),

            TX_WITHDRAW => {
                let (value, note, proof): (u64, Note, Vec<u8>) =
                    Canon::decode(&mut source)?;

                contract.withdraw(value, note, proof);
            }

            TX_WITHDRAW_OBFUSCATED => {
                let (message, message_address, note, proof): (
                    Message,
                    StealthAddress,
                    Note,
                    Vec<u8>,
                ) = Canon::decode(&mut source)?;

                contract.withdraw_obfuscated(
                    message,
                    message_address,
                    note,
                    proof,
                );
            }

            TX_WITHDRAW_TO_CONTRACT => {
                let (to, value): (ContractId, u64) =
                    Canon::decode(&mut source)?;

                contract.withdraw_to_contract(to, value);
            }

            _ => panic!("Tx id not implemented"),
        }

        let mut sink = Sink::new(&mut bytes[..]);

        ContractState::from_canon(&contract).encode(&mut sink);
        ReturnValue::from_canon(&true).encode(&mut sink);

        Ok(())
    }

    #[no_mangle]
    fn t(bytes: &mut [u8; PAGE_SIZE]) {
        // todo, handle errors here
        transaction(bytes).unwrap()
    }
}
