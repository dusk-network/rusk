// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

pub type Hash = [u8; 32];

#[derive(Default, Debug, Clone)]
pub struct Block {
    header: Header,
    txs: Vec<Transaction>,
    faults: Vec<Fault>,
}

impl PartialEq<Self> for Block {
    fn eq(&self, other: &Self) -> bool {
        self.header.hash == other.header.hash
    }
}

impl Eq for Block {}

impl Block {
    /// Creates a new block and calculates block hash, if missing.
    pub fn new(
        header: Header,
        txs: Vec<Transaction>,
        faults: Vec<Fault>,
    ) -> io::Result<Self> {
        let mut b = Block {
            header,
            txs,
            faults,
        };
        b.calculate_hash()?;
        Ok(b)
    }

    fn calculate_hash(&mut self) -> io::Result<()> {
        // Call hasher only if header.hash is empty
        if self.header.hash != Hash::default() {
            return Ok(());
        }

        let mut hasher = sha3::Sha3_256::new();
        self.header.marshal_hashable(&mut hasher)?;

        self.header.hash = hasher.finalize().into();
        Ok(())
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
    pub fn txs(&self) -> &Vec<Transaction> {
        &self.txs
    }
    pub fn faults(&self) -> &Vec<Fault> {
        &self.faults
    }

    pub fn set_attestation(&mut self, att: Attestation) {
        self.header.att = att;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Label {
    Accepted(u64),
    Attested(u64),
    Confirmed(u64),
    Final(u64),
}

/// Immutable view of a labelled block that is/(should be) persisted
#[derive(Debug, Clone)]
pub struct BlockWithLabel {
    blk: Block,
    label: Label,
}

impl BlockWithLabel {
    pub fn new_with_label(blk: Block, label: Label) -> Self {
        Self { blk, label }
    }
    pub fn inner(&self) -> &Block {
        &self.blk
    }
    pub fn label(&self) -> Label {
        self.label
    }
    pub fn is_final(&self) -> bool {
        matches!(self.label(), Label::Final(_)) || self.blk.header().height == 0
    }
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use super::*;
    use rand::Rng;
    use transaction::faker::gen_dummy_tx;

    impl<T> Dummy<T> for Block {
        /// Creates a block with 3 transactions and a random header.
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let txs = vec![
                gen_dummy_tx(rng.gen()),
                gen_dummy_tx(rng.gen()),
                gen_dummy_tx(rng.gen()),
            ];
            let header: Header = Faker.fake();
            let faults = vec![Faker.fake(), Faker.fake(), Faker.fake()];

            Block::new(header, txs, faults).expect("valid hash")
        }
    }
}
