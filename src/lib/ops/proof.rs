// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod bid_correctness;
use super::RuskExtenalError;
use super::RuskExternals;
use canonical_host::MemoryHolder;
use dusk_plonk::prelude::*;
use wasmi::{
    FuncInstance, FuncRef, RuntimeArgs, RuntimeValue, Signature, Trap,
    TrapKind, ValueType,
};

pub(crate) const INDEX: usize = 103;
pub(crate) const NAME: &'static str = "verify_proof";

/// Host call definition for the `VERIFY_SIG` opcode.
pub(crate) fn external(
    external: &mut RuskExternals,
    args: RuntimeArgs,
) -> Result<Option<RuntimeValue>, Trap> {
    match args.as_ref() {
        [RuntimeValue::I32(pub_inputs_len), RuntimeValue::I32(pub_inputs), RuntimeValue::I32(circuit_crate_len), RuntimeValue::I32(circuit_crate), RuntimeValue::I32(circuit_label_len), RuntimeValue::I32(circuit_label), RuntimeValue::I32(proof)] =>
        {
            let pub_inputs_len = *pub_inputs_len as usize;
            let pub_inputs = *pub_inputs as usize;
            let circuit_crate_len = *circuit_label_len as usize;
            let circuit_crate = *circuit_label as usize;
            let circuit_label_len = *circuit_label_len as usize;
            let circuit_label = *circuit_label as usize;
            let proof = *proof as usize;

            external.memory()?.with_direct_access_mut(|mem| {
                let pi: Vec<PublicInput> = mem[pub_inputs
                    ..pub_inputs
                        + pub_inputs_len * PublicInput::serialized_size()]
                    .chunks(PublicInput::serialized_size())
                    .map(|chunk| {
                        PublicInput::from_bytes(chunk).map_err(|_| {
                            Trap::new(TrapKind::Host(Box::new(
                                RuskExtenalError::WrongArgsNumber,
                            )))
                        })
                    })
                    .collect::<Result<_, Trap>>()?;

                let circuit_crate = String::from_utf8(
                    mem[circuit_crate..circuit_crate + circuit_crate_len]
                        .into(),
                )
                .map_err(|_| {
                    Trap::new(TrapKind::Host(Box::new(
                        RuskExtenalError::InvalidFFIEncoding,
                    )))
                })?;

                let circuit_label = String::from_utf8(
                    mem[circuit_label..circuit_label + circuit_label_len]
                        .into(),
                )
                .map_err(|_| {
                    Trap::new(TrapKind::Host(Box::new(
                        RuskExtenalError::InvalidFFIEncoding,
                    )))
                })?;

                let keys = rusk_profile::keys_for(circuit_crate.as_str());
                let (_, vk) = keys.get(circuit_label.as_str()).ok_or(
                    Trap::new(TrapKind::Host(Box::new(
                        RuskExtenalError::InvalidFFIEncoding,
                    ))),
                )?;

                let proof = Proof::from_bytes(
                    &mem[proof..proof + Proof::serialised_size()],
                )
                .map_err(|_| {
                    Trap::new(TrapKind::Host(Box::new(
                        RuskExtenalError::InvalidFFIEncoding,
                    )))
                })?;

                Ok(Some(RuntimeValue::I32(1i32)))
            })
        }

        _ => Err(Trap::new(TrapKind::Host(Box::new(
            RuskExtenalError::WrongArgsNumber,
        )))),
    }
    /*
    if let [RuntimeValue::I32(pub_inp_len), RuntimeValue::I32(pub_inp), RuntimeValue::I32(proof), RuntimeValue::I32(verif_key)] =
        args.as_ref()[..]
    {
        let pub_inp = pub_inp as usize;
        let pub_inp_len = pub_inp_len as usize;
        let proof = proof as usize;
        let verifier_key = verif_key as usize;
        external.memory()?.with_direct_access_mut(|mem| {
            // Build Public Inputs vector
            let mut pi_bytes =
                vec![0u8; pub_inp_len * PublicInput::serialized_size()];
            pi_bytes.copy_from_slice(
                &mem[pub_inp
                    ..pub_inp + pub_inp_len * PublicInput::serialized_size()],
            );
            let pi_vec = pi_bytes[..]
                .chunks(PublicInput::serialized_size())
                .map(|chunk| {
                    PublicInput::from_bytes(chunk).map_err(|_| {
                        Trap::new(TrapKind::Host(Box::new(
                            RuskExtenalError::WrongArgsNumber,
                        )))
                    })
                })
                .collect::<Result<Vec<PublicInput>, Trap>>()?;

            // Get the Proof from the memory bytes repr.
            let proof = Proof::from_bytes(
                &mem[proof..proof + Proof::serialised_size()],
            )
            .map_err(|_| {
                Trap::new(TrapKind::Host(Box::new(
                    RuskExtenalError::InvalidFFIEncoding,
                )))
            })?;

            let vk =
                VerifierKey::from_bytes(&mem[verifier_key..verifier_key + 728])
                    .map_err(|_| {
                        Trap::new(TrapKind::Host(Box::new(
                            RuskExtenalError::InvalidFFIEncoding,
                        )))
                    })?;
            // TODO: Check the Hash of VerifierKey and build the appropiate
            // circuit. For now we just execute the
            // bid_correctness_verification.
            bid_correctness::bid_correctness_verification(&pi_vec, &vk, &proof)
        })
    } else {
        Err(Trap::new(TrapKind::Host(Box::new(
            RuskExtenalError::WrongArgsNumber,
        ))))
    }
    */
}

#[inline]
pub(crate) fn wasmi_signature() -> FuncRef {
    FuncInstance::alloc_host(
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
        INDEX,
    )
}
