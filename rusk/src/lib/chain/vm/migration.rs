// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use dusk_consensus::user::provisioners::Provisioners;
use phoenix_core::transaction::StakeData;
use rusk_abi::{ContractData, ContractId, Error, Session, STAKE_CONTRACT};
use std::io;
use std::sync::mpsc;
use std::time::Instant;
use tracing::info;

const MIGRATION_GAS_LIMIT: u64 = 1_000_000_000;

const NEW_STAKE_CONTRACT_BYTECODE: &[u8] =
    include_bytes!("../../../assets/stake_contract.wasm");

pub struct Migration;

impl Migration {
    pub fn migrate(
        session: Session,
        provisioners: &Provisioners,
    ) -> crate::Result<Session> {
        info!("MIGRATING STAKE CONTRACT");
        let start = Instant::now();
        let mut session = session.migrate(
            STAKE_CONTRACT,
            NEW_STAKE_CONTRACT_BYTECODE,
            ContractData::builder(),
            MIGRATION_GAS_LIMIT,
            |new_contract, session| {
                Self::migrate_stakes(new_contract, session, provisioners)
            },
        )?;
        info!(
            "MIGRATION FINISHED: {:?}",
            Instant::now().duration_since(start)
        );

        info!("Performing sanity checks");

        let start = Instant::now();
        let new_list = Self::query_provisioners(&mut session)?;
        info!("Get new list: {:?}", Instant::now().duration_since(start));

        let start = Instant::now();
        let old_list = Self::old_provisioners(provisioners);

        // Assert both new_list and provisioner_list are identical
        if let Some((a, b)) = new_list.zip(old_list).find(|(a, b)| (a != b)) {
            tracing::error!("new = {a:?}");
            tracing::error!("old = {b:?}");
            Err(io::Error::new(io::ErrorKind::Other, "Wrong migration"))?;
        }
        info!(
            "Sanity checks OK: {:?}",
            Instant::now().duration_since(start)
        );

        Ok(session)
    }

    fn migrate_stakes(
        new_contract: ContractId,
        session: &mut Session,
        provisioners: &Provisioners,
    ) -> Result<(), Error> {
        for (pk, stake_data) in Self::old_provisioners(provisioners) {
            session.call::<_, ()>(
                new_contract,
                "insert_stake",
                &(pk, stake_data.clone()),
                MIGRATION_GAS_LIMIT,
            )?;
        }
        Ok(())
    }

    fn query_provisioners(
        session: &mut Session,
    ) -> crate::Result<impl Iterator<Item = (PublicKey, StakeData)>> {
        let (sender, receiver) = mpsc::channel();

        session.feeder_call::<_, ()>(STAKE_CONTRACT, "stakes", &(), sender)?;
        Ok(receiver.into_iter().map(|bytes| {
            rkyv::from_bytes::<(PublicKey, StakeData)>(&bytes).expect(
                "The contract should only return (pk, stake_data) tuples",
            )
        }))
    }

    fn old_provisioners(
        provisioners: &Provisioners,
    ) -> impl Iterator<Item = (PublicKey, StakeData)> + '_ {
        provisioners.iter().map(|(pk, stake)| {
            let amount = match stake.value() {
                0 => None,
                value => Some((value, stake.eligible_since)),
            };
            (
                *pk.inner(),
                StakeData {
                    amount,
                    reward: stake.reward,
                    counter: stake.counter,
                },
            )
        })
    }
}
