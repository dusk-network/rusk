// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod errors;
mod hashing;
mod proof;
mod sign;

use canonical_host::MemoryHolder;
pub use errors::RuskExtenalError;
use wasmi::{
    Error, Externals, FuncRef, MemoryRef, ModuleImportResolver, RuntimeArgs,
    RuntimeValue, Signature, Trap, TrapKind,
};

#[derive(Debug, Clone)]
pub struct RuskExternals {
    memory: Option<MemoryRef>,
}

impl Default for RuskExternals {
    fn default() -> Self {
        Self { memory: None }
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
            hashing::INDEX => hashing::external(self, args),
            sign::INDEX => sign::external(self, args),
            proof::INDEX => proof::external(self, args),
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
            hashing::NAME => Ok(hashing::wasmi_signature()),
            sign::NAME => Ok(sign::wasmi_signature()),
            proof::NAME => Ok(proof::wasmi_signature()),
            _ => Err(Error::Host(Box::new(
                RuskExtenalError::ResolverNameNotFound(field_name.to_string()),
            ))),
        }
    }
}
