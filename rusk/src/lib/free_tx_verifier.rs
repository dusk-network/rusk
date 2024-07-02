// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::free_tx_verifier::Error::*;
use crate::pow_verifier::PoW;
use node::database::DBViewer;
use node_data::ledger::Transaction;
use std::fmt;
use tracing::info;

const FRESHNESS: u64 = 100;
const POW_DIFFICULTY: usize = 16;
const BLOCK_HEIGHT_LEN: usize = std::mem::size_of::<u64>();

#[derive(Debug)]
pub enum Error {
    BlockHeightNotFound,
    BlockHeightInvalid,
    BlockHeightExpired,
    BlockNotFound,
    PoWInvalid,
    Database(anyhow::Error),
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Database(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub struct FreeTxVerifier;

impl FreeTxVerifier {
    pub fn verify(
        db_viewer: &dyn DBViewer,
        tx: &Transaction,
    ) -> Result<(), Error> {
        let tx = &tx.inner;
        let tip = db_viewer.fetch_tip_height()?;
        let user_height = Self::extract_block_id(&tx.proof)?;
        if user_height >= tip {
            return Err(BlockHeightInvalid);
        }
        if tip - user_height > FRESHNESS {
            return Err(BlockHeightExpired);
        }
        let block_hash = match db_viewer.fetch_block_hash(user_height)? {
            Some(block_hash) => block_hash,
            _ => return Err(BlockNotFound),
        };
        let mut bytes = tx.to_hash_input_bytes();
        bytes.extend(block_hash.as_ref());
        if !PoW::verify(bytes, &tx.proof[BLOCK_HEIGHT_LEN..], POW_DIFFICULTY) {
            info!("PoW invalid");
            return Err(PoWInvalid);
        }
        info!("PoW OK");
        Ok(())
    }

    fn extract_block_id(proof: impl AsRef<[u8]>) -> Result<u64, Error> {
        if proof.as_ref().len() < BLOCK_HEIGHT_LEN {
            Err(BlockHeightNotFound)
        } else {
            let mut value_bytes = [0; BLOCK_HEIGHT_LEN];
            value_bytes.copy_from_slice(&proof.as_ref()[..BLOCK_HEIGHT_LEN]);
            Ok(u64::from_le_bytes(value_bytes))
        }
    }
}
