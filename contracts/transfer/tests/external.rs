// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/*
const VERIFY_PROOF: usize = 103;

use canonical_host::MemoryHolder;
use wasmi::{
    Error as WasmiError, Externals, FuncInstance, FuncRef, MemoryRef,
    ModuleImportResolver, RuntimeArgs, RuntimeValue, Signature, Trap, TrapKind,
    ValueType,
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
        match (index, args.as_ref()) {
            (
                VERIFY_PROOF,
                [RuntimeValue::I32(pub_inputs_len), RuntimeValue::I32(pub_inputs), RuntimeValue::I32(circuit_crate_len), RuntimeValue::I32(circuit_crate), RuntimeValue::I32(circuit_label_len), RuntimeValue::I32(circuit_label), RuntimeValue::I32(proof)],
            ) => self.memory()?.with_direct_access_mut(|mem| {
                let pub_inputs_len = *pub_inputs_len as usize;
                let pub_inputs = *pub_inputs as usize;
                let circuit_crate_len = *circuit_label_len as usize;
                let circuit_crate = *circuit_label as usize;
                let circuit_label_len = *circuit_label_len as usize;
                let circuit_label = *circuit_label as usize;
                let proof = *proof as usize;

                Ok(Some(RuntimeValue::I32(1)))
            }),

            _ => Err(Trap::new(TrapKind::UnexpectedSignature)),
        }
    }
}

impl ModuleImportResolver for RuskExternals {
    fn resolve_func(
        &self,
        field_name: &str,
        _signature: &Signature,
    ) -> Result<FuncRef, WasmiError> {
        match field_name {
            "verify_proof" => Ok(FuncInstance::alloc_host(
                Signature::new(
                    &[
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                        ValueType::I32,
                    ][..],
                    Some(ValueType::I32),
                ),
                VERIFY_PROOF,
            )),

            _ => Err(WasmiError::Trap(TrapKind::UnexpectedSignature.into())),
        }
    }
}
*/
