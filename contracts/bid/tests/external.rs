// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical_host::MemoryHolder;
use dusk_plonk::prelude::*;
use poseidon252::sponge::hash;
use schnorr::single_key::{PublicKey, Signature as SchnorrSignature};
use wasmi::{
    Error, Externals, FuncRef, MemoryRef, ModuleImportResolver, RuntimeArgs,
    RuntimeValue, Signature, Trap,
};

const P_HASH: usize = 101;
const VERIFY_SIG: usize = 102;
const VERIFY_PROOF: usize = 103;

#[derive(Copy, Clone)]
pub struct RuskExternals {
    mem: Option<MemoryRef>,
}

impl MemoryHolder for RuskExternals {
    fn set_memory(&mut self, memory: MemoryRef) {
        self.mem = Some(memory)
    }
    fn memory(&self) -> Result<MemoryRef, wasmi::Trap> {
        Ok(self.mem.unwrap())
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
                    let ofs = ofs as usize;
                    let len = len as usize;
                    let ret_addr = ret_addr as usize;
                    mem[ret_addr..ret_addr + 32]
                        .copy_from_slice(&BlsScalar::one());
                    Ok(None)
                } else {
                    todo!("error out for wrong argument types")
                }
            }
            VERIFY_SIG => {
                if let [wasmi::RuntimeValue::I32(pk), wasmi::RuntimeValue::I32(sig), wasmi::RuntimeValue::I32(msg), wasmi::RuntimeValue::I32(ret_addr)] =
                    args.as_ref()[..]
                {
                    let pk = pk as usize;
                    let sig = sig as usize;
                    let msg = msg as usize;
                    let ret_addr = ret_addr as usize;
                    mem[ret_addr] = 1u8;
                    Ok(None)
                } else {
                    todo!("error out for wrong argument types")
                }
            }
            VERIFY_PROOF => {
                if let [wasmi::RuntimeValue::I32(pub_inp_len), wasmi::RuntimeValue::I32(pub_inp), wasmi::RuntimeValue::I32(proof), wasmi::RuntimeValue::I32(verif_key)] =
                    args.as_ref()[..]
                {
                    let pub_inp = pub_inp as usize;
                    let pub_inp_len = pub_inp_len as usize;
                    let proof = proof as usize;
                    let verifier_key = verif_key as usize;
                    Ok(Some(1usize))
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
            "p_hash" => Ok(wasmi::FuncInstance::alloc_host(
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
            "verify_sig" => Ok(wasmi::FuncInstance::alloc_host(
                wasmi::Signature::new(
                    &[
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                    ][..],
                    None,
                ),
                VERIFY_SIG,
            )),
            "verify_proof" => Ok(wasmi::FuncInstance::alloc_host(
                wasmi::Signature::new(
                    &[
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                        wasmi::ValueType::I32,
                    ][..],
                    Some(wasmi::ValueType::I32),
                ),
                VERIFY_BID_PROOF,
            )),
            _ => panic!("Unknown Rusk host fn {}", field_name),
        }
    }
}
