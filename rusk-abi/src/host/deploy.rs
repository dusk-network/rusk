// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use execution_core::transfer::Transaction;
use execution_core::ContractError;
use piecrust::{CallReceipt, Error as PiecrustError, Session};

use crate::gen_contract_id;

// Contract deployment will fail and charge full gas limit in the
// following cases:
// 1) Transaction gas limit is smaller than deploy charge plus gas used for
//    spending funds.
// 2) Transaction's bytecode's bytes are not consistent with bytecode's hash.
// 3) Deployment fails for deploy-specific reasons like e.g.:
//      - contract already deployed
//      - corrupted bytecode
//      - sufficient gas to spend funds yet insufficient for deployment
pub(crate) fn contract(
    session: &mut Session,
    tx: &Transaction,
    gas_per_deploy_byte: u64,
    receipt: &mut CallReceipt<Result<Vec<u8>, ContractError>>,
) {
    if let Some(deploy) = tx.deploy() {
        if receipt.data.is_ok() {
            let gas_left = tx.gas_limit() - receipt.gas_spent;
            let deploy_charge = tx.deploy_charge(gas_per_deploy_byte);
            let min_gas_limit = receipt.gas_spent + deploy_charge;

            if gas_left < min_gas_limit {
                receipt.data = Err(ContractError::OutOfGas);
            } else if !deploy.bytecode.verify_hash() {
                receipt.data = Err(ContractError::Panic(
                    "failed bytecode hash check".into(),
                ))
            } else {
                let result = session.deploy_raw(
                    Some(gen_contract_id(
                        &deploy.bytecode.bytes,
                        deploy.nonce,
                        &deploy.owner,
                    )),
                    deploy.bytecode.bytes.as_slice(),
                    deploy.init_args.clone(),
                    deploy.owner.clone(),
                    gas_left,
                );
                match result {
                    // Should the gas spent by the INIT method charged too?
                    Ok(_) => receipt.gas_spent += deploy_charge,
                    Err(err) => {
                        let msg = format!("failed deployment: {err:?}");
                        receipt.data = Err(ContractError::Panic(msg))
                    }
                }
            }
        }
    }
}

pub(crate) fn pre_check(
    tx: &Transaction,
    gas_per_deploy_byte: u64,
    min_deployment_gas_price: u64,
) -> Result<(), PiecrustError> {
    if tx.deploy().is_some() {
        let deploy_charge = tx.deploy_charge(gas_per_deploy_byte);

        if tx.gas_price() < min_deployment_gas_price {
            return Err(PiecrustError::Panic(
                "gas price too low to deploy".into(),
            ));
        }
        if tx.gas_limit() < deploy_charge {
            return Err(PiecrustError::Panic(
                "not enough gas to deploy".into(),
            ));
        }
    }

    Ok(())
}
