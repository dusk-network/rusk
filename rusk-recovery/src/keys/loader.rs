// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use license_circuits::LicenseCircuit;
use transfer_circuits::*;

use crate::keys::PUB_PARAMS;
use crate::keys::{CircuitLoader, TRANSCRIPT_LABEL};

macro_rules! loader_impl {
    ($loader:ident, $circuit:ty, $circuit_name:expr) => {
        pub struct $loader;
        impl CircuitLoader for $loader {
            fn circuit_id(&self) -> &[u8; 32] {
                <$circuit>::circuit_id()
            }

            fn circuit_name(&self) -> &'static str {
                $circuit_name
            }

            fn compile_to_bytes(
                &self,
            ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
                let (prover, verifier) = Compiler::compile::<$circuit>(
                    &PUB_PARAMS,
                    &TRANSCRIPT_LABEL,
                )?;

                Ok((prover.to_bytes(), verifier.to_bytes()))
            }
        }
    };
}

loader_impl!(
    StctCircuitLoader,
    SendToContractTransparentCircuit,
    "SendToContractTransparent"
);
loader_impl!(
    WfctCircuitLoader,
    WithdrawFromTransparentCircuit,
    "WithdrawFromContractTransparent"
);
loader_impl!(
    StcoCircuitLoader,
    SendToContractObfuscatedCircuit,
    "SendToContractObfuscated"
);
loader_impl!(
    WfcoCircuitLoader,
    WithdrawFromObfuscatedCircuit,
    "WithdrawFromContractObfuscated"
);
loader_impl!(
    ExecOneTwoCircuitLoader,
    ExecuteCircuitOneTwo,
    "ExecuteOneTwo"
);
loader_impl!(
    ExecTwoTwoCircuitLoader,
    ExecuteCircuitTwoTwo,
    "ExecuteTwoTwo"
);
loader_impl!(
    ExecThreeTwoCircuitLoader,
    ExecuteCircuitThreeTwo,
    "ExecuteThreeTwo"
);
loader_impl!(
    ExecFourTwoCircuitLoader,
    ExecuteCircuitFourTwo,
    "ExecuteFourTwo"
);
loader_impl!(LicenseCircuitLoader, LicenseCircuit, "LicenseCircuit");
