// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::node::Rusk;
use crate::Result;

use std::sync::mpsc;

use bytecheck::CheckBytes;
use dusk_core::abi::{ContractId, StandardBufSerializer};
use node::vm::VMExecution;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};

impl Rusk {
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
        let mut results = Vec::with_capacity(1);
        self.query_seq(contract_id, call_name, call_arg, |r| {
            results.push(r);
            None
        })?;
        Ok(results.pop().unwrap())
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

        // For feeder queries we use the gas limit set in the config
        session.feeder_call::<_, ()>(
            contract_id,
            call_name,
            call_arg,
            self.feeder_gas_limit,
            feeder,
        )?;

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
