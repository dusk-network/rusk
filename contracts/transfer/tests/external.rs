// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical_host::MemoryHolder;
use wasmi::{
    Error as WasmiError, Externals, FuncRef, MemoryRef, ModuleImportResolver,
    RuntimeArgs, RuntimeValue, Signature, Trap, TrapKind,
};

#[derive(Debug, Default, Clone)]
pub struct RuskExternals {
    mem: Option<MemoryRef>,
}

impl MemoryHolder for RuskExternals {
    fn set_memory(&mut self, memory: MemoryRef) {
        self.mem = Some(memory)
    }

    fn memory(&self) -> Result<MemoryRef, Trap> {
        self.mem
            .to_owned()
            .ok_or(Trap::new(TrapKind::ElemUninitialized))
    }
}

impl Externals for RuskExternals {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        unimplemented!()
    }
}

impl ModuleImportResolver for RuskExternals {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, WasmiError> {
        unimplemented!()
    }
}
