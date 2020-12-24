// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::RuskExtenalError;
use super::RuskExternals;
use canonical_host::MemoryHolder;
use dusk_plonk::bls12_381::BlsScalar;
use schnorr::single_key::{PublicKey, Signature as SchnorrSignature};
use wasmi::{FuncRef, RuntimeArgs, RuntimeValue, Trap, TrapKind};

pub(crate) const INDEX: usize = 102;
pub(crate) const NAME: &'static str = "verify_schnorr_sig";

/// Host call definition for the `VERIFY_SIG` opcode.
pub(crate) fn external(
    external: &mut RuskExternals,
    args: RuntimeArgs,
) -> Result<Option<RuntimeValue>, Trap> {
    if let [wasmi::RuntimeValue::I32(pk), wasmi::RuntimeValue::I32(sig), wasmi::RuntimeValue::I32(msg)] =
        args.as_ref()[..]
    {
        let pk = pk as usize;
        let sig = sig as usize;
        let msg = msg as usize;
        external.memory()?.with_direct_access_mut(|mem| {
            // Build Pk
            let mut bytes32 = [0u8; 32];
            let mut bytes64 = [0u8; 64];
            bytes32[0..32].copy_from_slice(&mem[pk..pk + 32]);
            let pk = PublicKey::from_bytes(&bytes32).unwrap();
            // Build Sig
            bytes64[0..64].copy_from_slice(&mem[sig..sig + 64]);
            let sig = SchnorrSignature::from_bytes(&bytes64).unwrap();
            // Build Msg
            bytes32[0..32].copy_from_slice(&mem[msg..msg + 32]);
            let msg = BlsScalar::from_bytes(&bytes32).unwrap();
            // Perform the signature verification
            match sig.verify(&pk, msg) {
                Ok(()) => Ok(Some(RuntimeValue::I32(1))),
                _ => Ok(Some(RuntimeValue::I32(0))),
            }
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
            Some(wasmi::ValueType::I32),
        ),
        INDEX,
    )
}
