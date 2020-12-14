// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod errors;
mod hashing;
mod signatures;

use canonical_host::MemoryHolder;
use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
pub(crate) use errors::RuskExtenalError;
use wasmi::{
    Error, Externals, FuncRef, MemoryRef, ModuleImportResolver, RuntimeArgs,
    RuntimeValue, Signature, Trap, TrapKind,
};

pub(crate) const P_HASH: usize = 1;
pub(crate) const VERIFY_SIG: usize = 2;

#[derive(Clone)]
pub struct RuskExternals {
    pub_params: &'static PublicParameters,
    memory: Option<MemoryRef>,
}

impl RuskExternals {
    /// Generate a new RuskExternals instance.
    pub fn new() -> Self {
        RuskExternals {
            memory: None,
            pub_params: &crate::PUB_PARAMS,
        }
    }
}

impl MemoryHolder for RuskExternals {
    fn set_memory(&mut self, memory: wasmi::MemoryRef) {
        self.memory = Some(memory);
    }
    fn memory(&self) -> Result<wasmi::MemoryRef, wasmi::Trap> {
        self.memory
            .to_owned()
            .ok_or_else(|| Trap::new(TrapKind::ElemUninitialized))
    }
}

impl Externals for RuskExternals {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            P_HASH => hashing::external(self, args),
            VERIFY_SIG => signatures::external(self, args),
            _ => Err(Trap::new(TrapKind::Host(Box::new(
                RuskExtenalError::InvokeIdxNotFound(index),
            )))),
        }
    }
}

impl ModuleImportResolver for RuskExternals {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, Error> {
        match field_name {
            "p_hash" => Ok(hashing::resolver()),
            "verify_sig" => Ok(signatures::resolver()),
            _ => Err(Error::Host(Box::new(
                RuskExtenalError::ResolverNameNotFound(field_name.to_string()),
            ))),
        }
    }
}
