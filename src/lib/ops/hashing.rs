// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::RuskExtenalError;
use super::RuskExternals;
use canonical_host::MemoryHolder;
use dusk_plonk::bls12_381::BlsScalar;
use poseidon252::sponge::hash;
use wasmi::{FuncRef, RuntimeArgs, RuntimeValue, Trap, TrapKind};

pub(crate) const INDEX: usize = 101;
pub(crate) const NAME: &'static str = "p_hash";

/// Host call definition for the `P_HASH` opcode.
pub(crate) fn external(
    external: &mut RuskExternals,
    args: RuntimeArgs,
) -> Result<Option<RuntimeValue>, Trap> {
    if let [wasmi::RuntimeValue::I32(ofs), wasmi::RuntimeValue::I32(len), wasmi::RuntimeValue::I32(ret_addr)] =
        args.as_ref()[..]
    {
        let ofs = ofs as usize;
        let len = len as usize;
        let ret_addr = ret_addr as usize;
        external.memory()?.with_direct_access_mut(|mem| {
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
        Err(Trap::new(TrapKind::Host(Box::new(
            RuskExtenalError::WrongArgsNumber,
        ))))
    }
}

#[inline]
pub(crate) fn wasmi_signature() -> FuncRef {
    wasmi::FuncInstance::alloc_host(
        wasmi::Signature::new(
            &[
                wasmi::ValueType::I32,
                wasmi::ValueType::I32,
                wasmi::ValueType::I32,
            ][..],
            None,
        ),
        INDEX,
    )
}
