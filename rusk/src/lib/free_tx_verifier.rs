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

pub struct FreeTxVerifier<'a> {
    db_viewer: &'a dyn DBViewer,
}

impl<'a> FreeTxVerifier<'a> {
    pub fn new(db_viewer: &'a dyn DBViewer) -> FreeTxVerifier<'a> {
        Self { db_viewer }
    }

    pub fn verify(&self, tx: &Transaction) -> Result<(), Error> {
        let tx = &tx.inner;
        let tip = self.db_viewer.fetch_tip_height()?;
        info!("top height={}", tip);
        let user_height = Self::extract_block_id(&tx.proof)?;
        info!("user height={}", user_height);
        if user_height >= tip {
            return Err(BlockHeightInvalid);
        }
        if tip - user_height > FRESHNESS {
            return Err(BlockHeightExpired);
        }
        let block_hash = match self.db_viewer.fetch_block_hash(user_height)? {
            Some(block_hash) => block_hash,
            _ => return Err(BlockNotFound),
        };
        info!("block hash={:x?}", block_hash);
        let mut bytes = tx.to_hash_input_bytes();
        bytes.extend(block_hash.as_ref());
        info!("about to verify PoW");
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
