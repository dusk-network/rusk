// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use dusk_consensus::user::provisioners::Provisioners;
use phoenix_core::transaction::StakeData;
use rusk_abi::{ContractData, ContractId, Error, Session, STAKE_CONTRACT};
use std::time::SystemTime;
use tracing::info;

const MIGRATION_GAS_LIMIT: u64 = 1_000_000_000;

const NEW_STAKE_CONTRACT_BYTECODE: &[u8] =
    include_bytes!("../../../assets/stake_contract.wasm");

pub struct Migration;

impl Migration {
    pub fn migrate(
        migration_height: Option<u64>,
        session: Session,
        block_height: u64,
        provisioners: &Provisioners,
    ) -> crate::Result<Session> {
        match migration_height {
            Some(h) if h == block_height => (),
            _ => return Ok(session),
        }
        info!("MIGRATING STAKE CONTRACT");
        let start = SystemTime::now();
        let session = session.migrate(
            STAKE_CONTRACT,
            NEW_STAKE_CONTRACT_BYTECODE,
            ContractData::builder(),
            MIGRATION_GAS_LIMIT,
            |new_contract, session| {
                Self::migrate_stakes(new_contract, session, provisioners)
            },
        )?;
        let stop = SystemTime::now();
        info!(
            "STAKE CONTRACT MIGRATION FINISHED: {:?}",
            stop.duration_since(start).expect("duration should work")
        );
        Ok(session)
    }

    fn migrate_stakes(
        new_contract: ContractId,
        session: &mut Session,
        provisioners: &Provisioners,
    ) -> Result<(), Error> {
        for (pk, stake_data) in Self::do_get_provisioners(provisioners)? {
            session.call::<_, ()>(
                new_contract,
                "insert_stake",
                &(pk, stake_data.clone()),
                MIGRATION_GAS_LIMIT,
            )?;
        }
        Ok(())
    }

    fn do_get_provisioners(
        provisioners: &Provisioners,
    ) -> anyhow::Result<impl Iterator<Item = (PublicKey, StakeData)> + '_> {
        Ok(provisioners.iter().map(|(pk, stake)| {
            (
                *pk.inner(),
                StakeData {
                    amount: Some((stake.value(), stake.eligible_since)),
                    reward: stake.reward,
                    counter: stake.counter,
                },
            )
        }))
    }
}
