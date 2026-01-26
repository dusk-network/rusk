// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::node::Rusk;
use crate::Result;
use std::any::{Any, TypeId};

use std::sync::mpsc;

use crate::node::rusk::TOOL_ACTIVE;
use bytecheck::CheckBytes;
use dusk_bytes::Serializable;
use dusk_core::abi::{ContractId, StandardBufSerializer};
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::phoenix::NoteOpening;
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_core::BlsScalar;
use dusk_vm::ContractMetadata;
use dusk_vm::Error::ContractDoesNotExist;
use node::vm::VMExecution;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

impl Rusk {
    pub fn query_metadata(
        &self,
        contract_id: &ContractId,
    ) -> Result<ContractMetadata> {
        let mut session = self.query_session(None)?;
        let metadata = session
            .contract_metadata(contract_id)
            .ok_or(ContractDoesNotExist(*contract_id))?;
        Ok(ContractMetadata {
            contract_id: metadata.contract_id,
            owner: metadata.owner.clone(),
        })
    }

    pub fn query_raw<S, V>(
        &self,
        contract_id: ContractId,
        fn_name: S,
        fn_arg: V,
    ) -> Result<Vec<u8>>
    where
        S: AsRef<str>,
        V: Into<Vec<u8>>,
    {
        if TOOL_ACTIVE && contract_id == TRANSFER_CONTRACT {
            println!("QUERY_RAW to {}", fn_name.as_ref());
        }
        let mut session = self.query_session(None)?;

        session
            .call_raw(
                contract_id,
                fn_name.as_ref(),
                fn_arg,
                self.get_block_gas_limit(),
            )
            .map(|receipt| receipt.data)
            .map_err(Into::into)
    }

    pub fn query<A, R>(
        &self,
        contract_id: ContractId,
        call_name: &str,
        call_arg: &A,
    ) -> Result<R>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> bytecheck::CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        if TOOL_ACTIVE && contract_id == TRANSFER_CONTRACT {
            println!("QUERY to {}", call_name);
        }
        let mut results = Vec::with_capacity(1);
        self.query_seq(contract_id, call_name, call_arg, |r| {
            results.push(r);
            None
        })?;
        Ok(results.pop().unwrap())
    }

    pub fn query_existing_nullifiers(
        &self,
        nullifiers: &Vec<BlsScalar>,
    ) -> Result<Vec<BlsScalar>> {
        if TOOL_ACTIVE {
            let transfer_tool = self.transfer_state.lock().unwrap();
            Ok(transfer_tool.existing_nullifiers(nullifiers.clone())) // todo: clone
        } else {
            self.query::<_, Vec<BlsScalar>>(
                TRANSFER_CONTRACT,
                "existing_nullifiers",
                nullifiers,
            )
        }
    }

    pub fn query_opening(&self, pos: u64) -> Result<Option<NoteOpening>> {
        if TOOL_ACTIVE {
            let transfer_tool = self.transfer_state.lock().unwrap();
            Ok(transfer_tool.opening(pos))
        } else {
            self.query::<_, Option<NoteOpening>>(
                TRANSFER_CONTRACT,
                "opening",
                &pos,
            )
        }
    }

    pub fn query_contract_balance(&self, id: &ContractId) -> Result<u64> {
        if TOOL_ACTIVE {
            let transfer_tool = self.transfer_state.lock().unwrap();
            Ok(transfer_tool.contract_balance(id))
        } else {
            self.query::<_, u64>(TRANSFER_CONTRACT, "contract_balance", id)
        }
    }

    pub fn query_root(&self) -> Result<BlsScalar> {
        if TOOL_ACTIVE {
            let transfer_tool = self.transfer_state.lock().unwrap();
            Ok(transfer_tool.root())
        } else {
            self.query::<_, BlsScalar>(TRANSFER_CONTRACT, "root", &())
        }
    }

    pub fn query_account(&self, pk: &AccountPublicKey) -> Result<AccountData> {
        if TOOL_ACTIVE {
            let transfer_tool = self.transfer_state.lock().unwrap();
            // println!("query_account {}",
            // bs58::encode(pk.to_bytes()).into_string());
            Ok(transfer_tool.account(pk))
        } else {
            self.query::<_, AccountData>(TRANSFER_CONTRACT, "account", pk)
        }
    }

    pub fn query_chain_id(&self) -> Result<u8> {
        if TOOL_ACTIVE {
            let transfer_tool = self.transfer_state.lock().unwrap();
            Ok(transfer_tool.chain_id())
        } else {
            self.query::<_, u8>(TRANSFER_CONTRACT, "chain_id", &())
        }
    }

    fn query_seq<A, R, F>(
        &self,
        contract_id: ContractId,
        call_name: &str,
        call_arg: &A,
        mut closure: F,
    ) -> Result<()>
    where
        F: FnMut(R) -> Option<A>,
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> bytecheck::CheckBytes<DefaultValidator<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        let mut session = self.query_session(None)?;

        let mut result = session
            .call(contract_id, call_name, call_arg, self.get_block_gas_limit())?
            .data;

        while let Some(call_arg) = closure(result) {
            result = session
                .call(
                    contract_id,
                    call_name,
                    &call_arg,
                    self.get_block_gas_limit(),
                )?
                .data;
        }

        session.call::<_, ()>(
            contract_id,
            call_name,
            call_arg,
            self.get_block_gas_limit(),
        )?;

        Ok(())
    }

    pub fn feeder_query<A>(
        &self,
        contract_id: ContractId,
        call_name: &str,
        call_arg: &A,
        feeder: mpsc::Sender<Vec<u8>>,
        base_commit: Option<[u8; 32]>,
    ) -> Result<()>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        A::Archived: for<'b> bytecheck::CheckBytes<DefaultValidator<'b>>,
    {
        let mut session = self.query_session(base_commit)?;

        if TOOL_ACTIVE
            && contract_id == TRANSFER_CONTRACT
            && call_name == "leaves_from_height"
        {
            let transfer_tool = self.transfer_state.lock().unwrap();
            let height: u64 = {
                unsafe { std::ptr::read(call_arg as *const A as *const u64) }
            };
            transfer_tool.leaves_from_height(height, feeder);
        } else {
            // For feeder queries we use the gas limit set in the config
            session.feeder_call::<_, ()>(
                contract_id,
                call_name,
                call_arg,
                self.feeder_gas_limit,
                feeder,
            )?;
        }

        Ok(())
    }

    pub fn feeder_query_raw<S, V>(
        &self,
        contract_id: ContractId,
        call_name: S,
        call_arg: V,
        feeder: mpsc::Sender<Vec<u8>>,
    ) -> Result<()>
    where
        S: AsRef<str>,
        V: Into<Vec<u8>>,
    {
        if TOOL_ACTIVE && contract_id == TRANSFER_CONTRACT {
            println!("FEEDER QUERY RAW to {}", call_name.as_ref());
        }
        let mut session = self.query_session(None)?;

        // For feeder queries we use the gas limit set in the config
        session.feeder_call_raw(
            contract_id,
            call_name.as_ref(),
            call_arg,
            self.feeder_gas_limit,
            feeder,
        )?;

        Ok(())
    }
}
