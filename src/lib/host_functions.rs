// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use poseidon252::sponge::hash;
use schnorr::single_key::{PublicKey, Signature as SchnorrSignature};
use wasmi::{
    Error, Externals, FuncRef, MemoryRef, ModuleImportResolver, RuntimeArgs,
    RuntimeValue, Signature, Trap,
};

const P_HASH: usize = 1;
const VERIFY_SIG: usize = 2;

#[derive(Copy, Clone)]
pub struct RuskExternals<'a> {
    pub_params: &'static PublicParameters,
    memory: &'a MemoryRef,
}

impl<'a> RuskExternals<'a> {
    /// Generate a new RuskExternals instance.
    pub fn new(memory: &'a wasmi::MemoryRef) -> Self {
        RuskExternals {
            memory,
            pub_params: &crate::PUB_PARAMS,
        }
    }
}

impl<'a> Externals for RuskExternals<'a> {
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
                    self.memory.with_direct_access_mut(|mem| {
                        let bytes = &mem[ofs..ofs + len];
                        // Chunk bytes to BlsSclar byte-size
                        let inp: Vec<BlsScalar> = bytes
                            .chunks(32usize)
                            .map(|scalar_bytes| {
                                let mut array = [0u8; 32];
                                array.copy_from_slice(&scalar_bytes[..]);
                                BlsScalar::from_bytes(&array).unwrap()
                            })
                            .collect();
                        let result = hash(&inp);
                        mem[ret_addr..ret_addr + 32]
                            .copy_from_slice(&result.to_bytes()[..]);
                        // Read Scalars from Chunks
                        Ok(None)
                    })
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
                    self.memory.with_direct_access_mut(|mem| {
                        // Build Pk
                        let mut bytes32 = [0u8; 32];
                        let mut bytes64 = [0u8; 64];
                        bytes32[0..32].copy_from_slice(&mem[pk..pk + 32]);
                        let pk = PublicKey::from_bytes(&bytes32).unwrap();
                        // Build Sig
                        bytes64[0..64].copy_from_slice(&mem[sig..sig + 64]);
                        let sig =
                            SchnorrSignature::from_bytes(&bytes64).unwrap();
                        // Build Msg
                        bytes32[0..32].copy_from_slice(&mem[msg..msg + 32]);
                        let msg = BlsScalar::from_bytes(&bytes32).unwrap();
                        // Perform the signature verification
                        match sig.verify(&pk, msg) {
                            Ok(()) => mem[ret_addr] = 1u8,
                            _ => mem[ret_addr] = 0u8,
                        };
                        Ok(None)
                    })
                } else {
                    todo!("error out for wrong argument types")
                }
            }
            _ => panic!("Unknown Rusk host fn {}", index),
        }
    }
}

impl<'a> ModuleImportResolver for RuskExternals<'a> {
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
            _ => panic!("Unknown Rusk host fn {}", field_name),
        }
    }
}
