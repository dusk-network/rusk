// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]
#![feature(arbitrary_self_types)]

extern crate alloc;

use rusk_abi::dusk::*;
use rusk_abi::ModuleId;
use rusk_abi::RawTransaction;
use transfer_contract_types::{Wfco2, Wfct2, Wfctc};
use dusk_pki::StealthAddress;

#[derive(Debug, Clone)]
pub struct Alice {
    transfer: ModuleId,
}

impl Alice {
    pub const fn new(transfer: ModuleId) -> Self {
        Self { transfer }
    }

    pub const fn identifier() -> &'static [u8; 5] {
        b"alice"
    }
}

#[cfg(target_family = "wasm")]
#[path = ""]
mod wasm {
    use super::*;

    use alloc::vec::Vec;
    use phoenix_core::{Message, Note};
    use rusk_abi::{ModuleId, PaymentInfo, State};

    const PAGE_SIZE: usize = 1024 * 4;

    #[no_mangle]
    static SELF_ID: ModuleId = ModuleId::uninitialized();

    static mut STATE: State<Alice> = State::new(Alice::new(SELF_ID));

    #[no_mangle]
    unsafe fn ping(arg_len: u32) -> u32 {
        rusk_abi::wrap_query(
            arg_len,
            |()| {
                STATE.ping()
            },
        )
    }

    #[no_mangle]
    unsafe fn withdraw(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(
            arg_len,
            |(value, note, proof)| {
                STATE.withdraw(value, note, proof)
            },
        )
    }

    #[no_mangle]
    unsafe fn withdraw_obfuscated(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(
            arg_len,
            |(message, message_address, change, change_address, output, proof)| {
                STATE.withdraw_obfuscated(message, message_address, change, change_address, output, proof)
            },
        )
    }

    #[no_mangle]
    unsafe fn withdraw_to_contract(arg_len: u32) -> u32 {
        rusk_abi::wrap_transaction(
            arg_len,
            |(to, value)| {
                STATE.withdraw_to_contract(to, value)
            },
        )
    }


    impl Alice {
        pub fn ping(&mut self) {
            rusk_abi::debug!("Alice ping");
        }

        pub fn withdraw(self: &mut State<Self>, value: u64, note: Note, proof: Vec<u8>) {
            let transfer = self.transfer;
            let transaction = RawTransaction::new(
                "withdraw",
                Wfct2 {
                    value,
                    note,
                    proof,
                },
            );
            self.transact_raw(transfer, transaction)
                .expect("Failed to withdraw");
       }

        pub fn withdraw_obfuscated(
            self: &mut State<Self>,
            message: Message,
            message_address: StealthAddress,
            change: Message,
            change_address: StealthAddress,
            output: Note,
            proof: Vec<u8>,
        ) {
            let transfer = self.transfer;
            let transaction = RawTransaction::new(
                "withdraw_obfuscated",
                Wfco2 {
                    message,
                    message_address,
                    change,
                    change_address,
                    output,
                    proof,
                });
            self.transact_raw(transfer, transaction)
                .expect("Failed to withdraw obfuscated!");
        }

        pub fn withdraw_to_contract(self: &mut State<Self>, to: ModuleId, value: u64) {
            let transfer = self.transfer;
            let transaction = RawTransaction::new(
                "withdraw_to_contract",
                Wfctc {
                    module: to,
                    value,
                });
            self.transact_raw(transfer, transaction)
                .expect("Failed to withdraw");
        }
    }

    // fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
    //     let mut source = Source::new(&bytes[..]);
    //
    //     let _contract = Alice::decode(&mut source)?;
    //     let qid = u8::decode(&mut source)?;
    //
    //     match qid {
    //         rusk_abi::PAYMENT_INFO => {
    //             let ret = PaymentInfo::Any(None);
    //
    //             let r = {
    //                 // return value
    //                 let wrapped_return = ReturnValue::from_canon(&ret);
    //
    //                 let mut sink = Sink::new(&mut bytes[..]);
    //
    //                 wrapped_return.encode(&mut sink)
    //             };
    //
    //             Ok(r)
    //         }
    //
    //         _ => panic!(""),
    //     }
    // }

    // #[no_mangle]
    // fn q(bytes: &mut [u8; PAGE_SIZE]) {
    //     let _ = query(bytes);
    // }

    // fn transaction(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
    //     let mut source = Source::new(bytes);
    //
    //     let mut contract = Alice::decode(&mut source)?;
    //     let tid = u8::decode(&mut source)?;
    //
    //     match tid {
    //         TX_PING => contract.ping(),
    //
    //         TX_WITHDRAW => {
    //             let (value, note, proof): (u64, Note, Vec<u8>) =
    //                 Canon::decode(&mut source)?;
    //
    //             contract.withdraw(value, note, proof);
    //         }
    //
    //         TX_WITHDRAW_OBFUSCATED => {
    //             let (
    //                 message,
    //                 message_address,
    //                 change,
    //                 change_address,
    //                 note,
    //                 proof,
    //             ): (
    //                 Message,
    //                 StealthAddress,
    //                 Message,
    //                 StealthAddress,
    //                 Note,
    //                 Vec<u8>,
    //             ) = Canon::decode(&mut source)?;
    //
    //             contract.withdraw_obfuscated(
    //                 message,
    //                 message_address,
    //                 change,
    //                 change_address,
    //                 note,
    //                 proof,
    //             );
    //         }
    //
    //         TX_WITHDRAW_TO_CONTRACT => {
    //             let (to, value): (ContractId, u64) =
    //                 Canon::decode(&mut source)?;
    //
    //             contract.withdraw_to_contract(to, value);
    //         }
    //
    //         _ => panic!("Tx id not implemented"),
    //     }
    //
    //     let mut sink = Sink::new(&mut bytes[..]);
    //
    //     ContractState::from_canon(&contract).encode(&mut sink);
    //     ReturnValue::from_canon(&true).encode(&mut sink);
    //
    //     Ok(())
    // }
    //
    // #[no_mangle]
    // fn t(bytes: &mut [u8; PAGE_SIZE]) {
    //     // todo, handle errors here
    //     transaction(bytes).unwrap()
    // }
}
