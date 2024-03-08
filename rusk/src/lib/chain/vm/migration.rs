// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey;
use phoenix_core::transaction::StakeData;
use rusk_abi::{ContractData, ContractId, Error, Session, STAKE_CONTRACT, VM};
use std::sync::mpsc;
use std::time::SystemTime;
use tracing::info;

const MIGRATION_GAS_LIMIT: u64 = 1_000_000_000;

const NEW_STAKE_CONTRACT_BYTECODE: &[u8] =
    include_bytes!("../../../assets/stake_contract.wasm");

pub struct Migration;

impl Migration {
    pub fn migrate(
        migration_height: Option<u64>,
        vm: &VM,
        current_commit: [u8; 32],
        block_height: u64,
    ) -> anyhow::Result<()> {
        match migration_height {
            Some(h) if h == block_height => (),
            _ => return Ok(()),
        }
        info!("MIGRATING STAKE CONTRACT");
        let mut session =
            rusk_abi::new_session(vm, current_commit, block_height)?;
        let start = SystemTime::now();
        session = session.migrate(
            STAKE_CONTRACT,
            NEW_STAKE_CONTRACT_BYTECODE,
            ContractData::builder(),
            MIGRATION_GAS_LIMIT,
            |new_contract, session| {
                Self::migrate_stakes(STAKE_CONTRACT, new_contract, session)
            },
        )?;
        Self::display_stake_contract_version(
            &mut session,
            "after_migration",
            MIGRATION_GAS_LIMIT,
        );
        let stop = SystemTime::now();
        let _root = session.commit()?;
        info!(
            "STAKE CONTRACT MIGRATION FINISHED: {:?}",
            stop.duration_since(start).expect("duration should work")
        );
        Ok(())
    }

    fn migrate_stakes(
        old_contract: ContractId,
        new_contract: ContractId,
        session: &mut Session,
    ) -> Result<(), Error> {
        for (pk, stake_data) in
            Self::do_get_provisioners(old_contract, session)?
        {
            session.call::<_, ()>(
                new_contract,
                "insert_stake",
                &(pk, stake_data),
                MIGRATION_GAS_LIMIT,
            )?;
        }
        let slashed_amount = session
            .call::<_, u64>(
                old_contract,
                "slashed_amount",
                &(),
                MIGRATION_GAS_LIMIT,
            )?
            .data;
        session.call::<_, ()>(
            new_contract,
            "set_slashed_amount",
            &slashed_amount,
            MIGRATION_GAS_LIMIT,
        )?;
        Ok(())
    }

    fn do_get_provisioners(
        contract_id: ContractId,
        session: &mut Session,
    ) -> anyhow::Result<impl Iterator<Item = (PublicKey, StakeData)>> {
        let (sender, receiver) = mpsc::channel();
        session.feeder_call::<_, ()>(contract_id, "stakes", &(), sender)?;
        Ok(receiver.into_iter().map(|bytes| {
            rkyv::from_bytes::<(PublicKey, StakeData)>(&bytes).expect(
                "The contract should only return (pk, stake_data) tuples",
            )
        }))
    }

    fn display_stake_contract_version(
        session: &mut Session,
        message: impl AsRef<str>,
        gas_limit: u64,
    ) {
        let v = session
            .call::<_, u64>(STAKE_CONTRACT, "get_version", &(), gas_limit)
            .expect("getting stake contract version should succeed")
            .data;
        info!(
            "CURRENT STAKE CONTRACT VERSION={} ({})",
            v,
            message.as_ref()
        );
    }
}
