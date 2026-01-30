// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::transfer::withdraw::Withdraw;
use dusk_core::transfer::{ContractToAccount, ContractToContract};
use dusk_data_driver::from_rkyv;
use piecrust_uplink::ContractId;
use std::sync::{Arc, Mutex};
use transfer::TransferState;

// todo: this class eventually is in prod, not test
pub struct TransferCallback;

impl TransferCallback {
    pub fn process(
        transfer_tool: Arc<Mutex<TransferState>>,
        contract_id: [u8; 32],
        fn_name: String,
        args: Vec<u8>,
        bh: u64,
    ) -> Vec<u8> {
        if fn_name == "deposit" {
            let value = from_rkyv(&args).expect("argument deserialization");
            let mut transfer_tool_guard = transfer_tool.lock().unwrap();
            let _r =
                transfer_tool_guard // todo: process result
                    .deposit(
                        value,
                        ContractId::from_bytes(contract_id),
                    );
        } else if fn_name == "withdraw" {
            let withdraw: Withdraw =
                from_rkyv(&args).expect("argument deserialization");
            let mut transfer_tool_guard = transfer_tool.lock().unwrap();
            let _r = transfer_tool_guard.withdraw(
                // todo: process result
                withdraw,
                bh,
                ContractId::from_bytes(contract_id),
            );
        } else if fn_name == "contract_to_contract" {
            let contract_to_contract: ContractToContract =
                from_rkyv(&args).expect("argument deserialization");
            let mut transfer_tool_guard = transfer_tool.lock().unwrap();
            let _r = transfer_tool_guard.contract_to_contract(
                // todo: process result
                contract_to_contract,
                ContractId::from_bytes(contract_id),
            );
        } else if fn_name == "contract_to_account" {
            let contract_to_account: ContractToAccount =
                from_rkyv(&args).expect("argument deserialization");
            let mut transfer_tool_guard = transfer_tool.lock().unwrap();
            let _r = transfer_tool_guard.contract_to_account(
                // todo: process result
                contract_to_account,
                ContractId::from_bytes(contract_id),
            );
        }
        // todo: process return argument
        vec![]
    }
}
