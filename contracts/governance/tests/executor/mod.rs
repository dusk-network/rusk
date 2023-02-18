// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod tx;

use dusk_abi::{ContractId, Transaction};
use dusk_pki::SecretSpendKey;
use governance_contract::GovernanceContract;
use std::error::Error;
use transfer_wrapper::TransferWrapper;

const GOVERNANCE_BYTECODE: &[u8] = include_bytes!(
    "../../../../target/wasm32-unknown-unknown/release/governance_contract.wasm"
);

pub struct Executor {
    genesis_ssk: SecretSpendKey,
    wrapper: TransferWrapper,
    contract_id: ContractId,
    block_heigth: u64,
}

impl Executor {
    pub fn new(
        seed: u64,
        contract: GovernanceContract,
        genesis_value: u64,
    ) -> Executor {
        let mut wrapper = TransferWrapper::new(seed, genesis_value);

        let (genesis_ssk, _) = wrapper.genesis_identifier();
        let contract_id = wrapper.deploy(contract, GOVERNANCE_BYTECODE);
        Executor {
            genesis_ssk,
            wrapper,
            contract_id,
            block_heigth: 0,
        }
    }

    pub fn state(&self) -> GovernanceContract {
        self.wrapper.state(&self.contract_id)
    }

    pub fn run(
        &mut self,
        transaction: Transaction,
    ) -> Result<GovernanceContract, Box<dyn Error>> {
        self.block_heigth += 1;

        let (unspent_notes, note_keys) =
            self.wrapper.unspent_notes(&self.genesis_ssk);

        let refund_vk = self.genesis_ssk.view_key();
        let refund_psk = self.genesis_ssk.public_spend_key();
        let remainder_psk = refund_psk;

        let gas_limit = 1_750_000_000;
        let gas_price = 1;
        let fee = self.wrapper.fee(gas_limit, gas_price, &refund_psk);

        self.wrapper.execute(
            self.block_heigth,
            &unspent_notes,
            &note_keys,
            &refund_vk,
            &remainder_psk,
            true,
            fee,
            None,
            Some((self.contract_id, transaction)),
        )?;
        Ok(self.state())
    }
}
