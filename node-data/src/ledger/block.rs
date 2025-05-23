// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;

pub type Hash = [u8; 32];
pub type Bloom = [u8; 256];

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

    pub fn size(&self) -> io::Result<usize> {
        let mut buf = vec![];
        self.write(&mut buf)?;
        Ok(buf.len())
    }

    pub fn set_signature(&mut self, signature: Signature) {
        self.header.signature = signature;
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

#[derive(Debug, Clone)]
/// Immutable view of a finalized block with spent transactions that is/(should
/// be) persisted
pub struct BlockWithSpentTransactions {
    header: Header,
    txs: Vec<SpentTransaction>,
    faults: Vec<Fault>,
    label: Label,
}

impl PartialEq<Self> for BlockWithSpentTransactions {
    fn eq(&self, other: &Self) -> bool {
        self.header.hash == other.header.hash
    }
}

impl Eq for BlockWithSpentTransactions {}

impl BlockWithSpentTransactions {
    /// Creates a new BlockWithSpentTransactions. The block is already finalized
    /// and has a hash and no faults.
    pub fn new(
        header: Header,
        txs: Vec<SpentTransaction>,
        faults: Vec<Fault>,
        label: Label,
    ) -> Self {
        BlockWithSpentTransactions {
            header,
            txs,
            faults,
            label,
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
    pub fn txs(&self) -> &Vec<SpentTransaction> {
        &self.txs
    }
    pub fn faults(&self) -> &Vec<Fault> {
        &self.faults
    }
    pub fn label(&self) -> &Label {
        &self.label
    }
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use rand::Rng;
    use transaction::faker::gen_dummy_tx;

    use super::*;

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
