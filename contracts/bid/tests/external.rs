// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical_host::MemoryHolder;
use dusk_plonk::prelude::*;
use wasmi::{
    Error, Externals, FuncRef, MemoryRef, ModuleImportResolver, RuntimeArgs,
    RuntimeValue, Signature, Trap, TrapKind,
};

const P_HASH: usize = 101;
const VERIFY_SCHNORR_SIG: usize = 102;
const VERIFY_PROOF: usize = 103;

#[derive(Debug, Clone)]
pub struct RuskExternals {
    pub mem: Option<MemoryRef>,
}

impl MemoryHolder for RuskExternals {
    fn set_memory(&mut self, memory: MemoryRef) {
        self.mem = Some(memory)
    }
    fn memory(&self) -> Result<wasmi::MemoryRef, wasmi::Trap> {
        self.mem
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
            P_HASH => {
                if let [wasmi::RuntimeValue::I32(ofs), wasmi::RuntimeValue::I32(len), wasmi::RuntimeValue::I32(ret_addr)] =
                    args.as_ref()[..]
                {
                    self.memory()?.with_direct_access_mut(|mem| {
                        let _ = ofs as usize;
                        let _ = len as usize;
                        let ret_addr = ret_addr as usize;
                        mem[ret_addr..ret_addr + 32]
                            .copy_from_slice(&BlsScalar::one().to_bytes()[..]);
                        Ok(None)
                    })
                } else {
                    panic!("No error handling is impl for a Dummy Resolver")
                }
            }
            VERIFY_SCHNORR_SIG => {
                if let [wasmi::RuntimeValue::I32(pk), wasmi::RuntimeValue::I32(sig), wasmi::RuntimeValue::I32(msg)] =
                    args.as_ref()[..]
                {
                    self.memory()?.with_direct_access_mut(|_| {
                        let _ = pk as usize;
                        let _ = sig as usize;
                        let _ = msg as usize;

                        Ok(Some(RuntimeValue::I32(1)))
                    })
                } else {
                    panic!("No error handling is impl for a Dummy Resolver")
                }
            }
            VERIFY_PROOF => {
                if let [wasmi::RuntimeValue::I32(pub_inp_len), wasmi::RuntimeValue::I32(pub_inp), wasmi::RuntimeValue::I32(proof), wasmi::RuntimeValue::I32(verif_key)] =
                    args.as_ref()[..]
                {
                    self.memory()?.with_direct_access_mut(|_| {
                        let _ = pub_inp as usize;
                        let _ = pub_inp_len as usize;
                        let _ = proof as usize;
                        let _ = verif_key as usize;
                        Ok(Some(RuntimeValue::I32(1i32)))
                    })
                } else {
                    panic!("No error handling is impl for a Dummy Resolver")
                }
            }
            _ => panic!("Unknown Rusk host fn {}", index),
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
            "_p_hash" => Ok(wasmi::FuncInstance::alloc_host(
                wasmi::Signature::new(
                    &[
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                    ][..],
                    None,
                ),
                P_HASH,
            )),
            "_verify_schnorr_sig" => Ok(wasmi::FuncInstance::alloc_host(
                wasmi::Signature::new(
                    &[
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                    ][..],
                    Some(wasmi::ValueType::I32),
                ),
                VERIFY_SCHNORR_SIG,
            )),
            "_verify_proof" => Ok(wasmi::FuncInstance::alloc_host(
                wasmi::Signature::new(
                    &[
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                    ][..],
                    Some(wasmi::ValueType::I32),
                ),
                VERIFY_PROOF,
            )),

            _ => panic!("Unknown Rusk host fn {}", field_name),
        }
    }
}
