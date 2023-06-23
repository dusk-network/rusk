// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{bls, Serializable};
use dusk_bytes::Serializable as DuskBytesSerializable;
use sha3::Digest;
use std::io::{self, Read, Write};

#[cfg(any(feature = "faker", test))]
use fake::{Dummy, Fake, Faker};

pub type Seed = Signature;
pub type Hash = [u8; 32];

#[derive(Default, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub txs: Vec<Transaction>,
}

#[derive(Default, Eq, PartialEq, Clone)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct Header {
    // Hashable fields
    pub version: u8,
    pub height: u64,
    pub timestamp: i64,
    pub prev_block_hash: Hash,
    pub seed: Seed,
    pub state_hash: Hash,
    pub generator_bls_pubkey: bls::PublicKeyBytes,
    pub txroot: Hash,
    pub gas_limit: u64,
    pub iteration: u8,

    // Block hash
    pub hash: Hash,

    // Non-hashable fields
    pub cert: Certificate,
}

impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let timestamp =
            chrono::NaiveDateTime::from_timestamp_opt(self.timestamp, 0)
                .map_or_else(
                    || "unknown".to_owned(),
                    |v| {
                        chrono::DateTime::<chrono::Utc>::from_utc(
                            v,
                            chrono::Utc,
                        )
                        .to_rfc2822()
                    },
                );

        f.debug_struct("Header")
            .field("version", &self.version)
            .field("height", &self.height)
            .field("timestamp", &timestamp)
            .field("prev_block_hash", &hex::encode(self.prev_block_hash))
            .field("seed", &hex::encode(self.seed.inner()))
            .field("state_hash", &hex::encode(self.state_hash))
            .field(
                "generator_bls_pubkey",
                &hex::encode(self.generator_bls_pubkey.inner()),
            )
            .field("gas_limit", &self.gas_limit)
            .field("hash", &hex::encode(self.hash))
            .field("cert", &self.cert)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub inner: phoenix_core::Transaction,
    pub gas_spent: Option<u64>,
    pub err: Option<String>
}

impl Transaction {
    pub fn hash(&self) -> [u8; 32] {
        rusk_abi::hash(self.inner.to_hash_input_bytes()).to_bytes()
    }
    pub fn gas_price(&self) -> u64 {
        self.inner.fee().gas_price
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct Certificate {
    pub first_reduction: StepVotes,
    pub second_reduction: StepVotes,
}

impl Header {
    /// Marshal hashable fields.
    ///
    /// Param `fixed_size_seed` changes the way seed is marshaled.
    /// In block hashing, header seed is fixed-size field while in wire
    /// message marshaling it is variable-length field.
    pub(crate) fn marshal_hashable<W: Write>(
        &self,
        w: &mut W,
        fixed_size_seed: bool,
    ) -> io::Result<()> {
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.height.to_le_bytes())?;
        w.write_all(&(self.timestamp as u64).to_le_bytes())?;
        w.write_all(&self.prev_block_hash[..])?;

        if fixed_size_seed {
            w.write_all(&self.seed.inner()[..])?;
        } else {
            Self::write_var_le_bytes(w, &self.seed.inner()[..])?;
        }

        w.write_all(&self.state_hash[..])?;
        w.write_all(&self.generator_bls_pubkey.inner()[..])?;
        w.write_all(&self.txroot[..])?;
        w.write_all(&self.gas_limit.to_le_bytes())?;
        w.write_all(&self.iteration.to_le_bytes())?;

        Ok(())
    }

    pub(crate) fn unmarshal_hashable<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;
        let version = buf[0];

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let height = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let timestamp = u64::from_le_bytes(buf) as i64;

        let mut prev_block_hash = [0u8; 32];
        r.read_exact(&mut prev_block_hash[..])?;

        let value = Self::read_var_le_bytes(r)?;
        let seed: [u8; 48] = value
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut state_hash = [0u8; 32];
        r.read_exact(&mut state_hash[..])?;

        let mut generator_bls_pubkey = [0u8; 96];
        r.read_exact(&mut generator_bls_pubkey[..])?;

        let mut txroot = [0u8; 32];
        r.read_exact(&mut txroot[..])?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let gas_limit = u64::from_le_bytes(buf);

        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;
        let iteration = buf[0];

        Ok(Header {
            version,
            height,
            timestamp,
            gas_limit,
            prev_block_hash,
            seed: Seed::from(seed),
            generator_bls_pubkey: bls::PublicKeyBytes(generator_bls_pubkey),
            iteration,
            state_hash,
            txroot,
            hash: [0; 32],
            cert: Default::default(),
        })
    }
}

impl Block {
    /// Creates a new block and calculates block hash, if missing.
    pub fn new(header: Header, txs: Vec<Transaction>) -> io::Result<Self> {
        let mut b = Block { header, txs };
        b.calculate_hash()?;
        Ok(b)
    }

    pub fn calculate_hash(&mut self) -> io::Result<()> {
        // Call hasher only if header.hash is empty
        if self.header.hash != Hash::default() {
            return Ok(());
        }

        let mut hasher = sha3::Sha3_256::new();
        self.header.marshal_hashable(&mut hasher, true)?;

        self.header.hash = hasher.finalize().into();
        Ok(())
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
    pub fn txs(&self) -> &Vec<Transaction> {
        &self.txs
    }
}

#[derive(Debug, Default, Clone, Eq, Hash, PartialEq)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct StepVotes {
    pub bitset: u64,
    pub signature: Signature,
}

impl StepVotes {
    pub fn new(signature: [u8; 48], bitset: u64) -> StepVotes {
        StepVotes {
            bitset,
            signature: Signature(signature),
        }
    }
}

/// a wrapper of 48-sized array to facilitate Signature
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Signature(pub [u8; 48]);

impl Signature {
    pub fn is_zeroed(&self) -> bool {
        self.0 == [0; 48]
    }
    pub fn inner(&self) -> [u8; 48] {
        self.0
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signature")
            .field("signature", &hex::encode(self.0))
            .finish()
    }
}

impl From<[u8; 48]> for Signature {
    fn from(value: [u8; 48]) -> Self {
        Self(value)
    }
}

impl Default for Signature {
    fn default() -> Self {
        Signature([0; 48])
    }
}

impl PartialEq<Self> for Block {
    fn eq(&self, other: &Self) -> bool {
        self.header.hash == other.header.hash
    }
}

impl Eq for Block {}

impl PartialEq<Self> for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash() && self.gas_spent == other.gas_spent
    }
}

impl Eq for Transaction {}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use super::*;
    use crate::bls::PublicKeyBytes;
    use hex;
    use rand::Rng;

    impl<T> Dummy<T> for Block {
        /// Creates a block with 3 transactions and random header.
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let txs = vec![
                gen_dummy_tx(rng.gen()),
                gen_dummy_tx(rng.gen()),
                gen_dummy_tx(rng.gen()),
            ];
            let header: Header = Faker.fake();

            Block::new(header, txs).expect("valid hash")
        }
    }

    impl<T> Dummy<T> for Transaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            gen_dummy_tx(1_000_000)
        }
    }

    impl<T> Dummy<T> for PublicKeyBytes {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let rand_val = rng.gen::<[u8; 32]>();
            let mut bls_key = [0u8; 96];
            bls_key[..32].copy_from_slice(&rand_val);
            bls::PublicKeyBytes(bls_key)
        }
    }

    impl<T> Dummy<T> for Signature {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, rng: &mut R) -> Self {
            let rand_val = rng.gen::<[u8; 32]>();
            let mut rand_signature = [0u8; 48];
            rand_signature[..32].copy_from_slice(&rand_val);

            Signature(rand_signature)
        }
    }

    /// Generates a decodable transaction from a fixed blob with a specified
    /// gas price.
    pub fn gen_dummy_tx(gas_price: u64) -> Transaction {
        // TODO: Replace this blob with making a valid transaction once
        // dusk_wallet_core::Transaction allows this
        let fixed = "010000000000000001020304050607080102030405060708010203040506070801020304050607080200000000000000010c8088b9e8c9d06915673d4d94fc76348fb7ce7503e8587f30caea67ab8379b815ce6aba274054f337bdd92d9411d8be3f282b05e3c6d42e8eea9f3215b8de33b96a3c7c1dbcb4d8cdd8ef13e50e84cf6480116311677676269d3e662cea608c5a3479e042102a78621252a37f2d99e6824e17a2b11597147d1adf4624e7d436ffffffffffffffff997ebe7877346dc48137c1d115176c60c5dbf0ea77dd8cdca0cfbc0f3d90304ecb5b2b3d60a2b9d4df4a999ef3a768f8bd75c75aac343bff35bed7cfb2e3513315e8ece73c24ca0c97bda403149dcf9fea1c8827b682c1bbe089c8d10355c45e01e549d068cb470cbefe6fddd3b2d8aacfa5a76805e725d5394e882a79d157695ec48dcb7e531ccc3b334ae122d4fd40e242e7d8a85fdb82bd4c9e9621a9a60d042dbbaec8a2acb879b48d311f1264b1aafe6bf26ccc0bb250af7a2e19e8dcdc3851f382c509fb449a701a93c9489ae97bae88feaebe38fc6c128dc4b286724c10ffffffffffffffff14b611da24f94e89dd03121410f05b53c52cfc785da3c8d59bb65d07b78ab0241c6a8f3ffadc251790b9f78a31b82246c883cbfa1330337bd094045c01dcca2a7de1eadf6f1f7116169ed9dd10541a407035bb8fe834a973d8176f51f07a8435fee6a01aa94b675366ed1b054b8091542329dd1538bcec8a7503906281f0b61200ca9a3b00000000GASPRICEd85dbd596fc0476c779f3e2e7b5e58b732cb71f9ca056a8828cf845885a22f17848a28b1224942eb4222b7c43fc01e60529c7ee5fab115f3802c91179d0edfa19851d4394c5da06a86f955b2bd1305672e61a9569b5e024f03c957d4160d3d23fad4651d0897d60d89845c58baee90dbb291366e711628910673b9f3eedaaec355d87e2b2619a6809157bf01d3579145794a2b10e5e0f23d053e48a699ad318d80d2e737ca67e32f0848724907f3a847befe125d83031fc249cc24d489bee3cca6dfba0129d5578102c594b72631a13797cc0413391a5a1886c7536e6fdc0c489dfdbc00baba13e05157a7ab7273523dbb98d34c06e3a058424f361aad4a8fbda04b3327dbf973a2fc07d54445ebe6651b2e35a3f5c983dad6f05599505d20e8049ab8b6a8f099304dbc4badb806e2e8b02f90619eacef17710c48c316cddd0889badea8613806d13450208797859e6271335cda185bbfc5844358e701c0ca03ad84e86019661d4b29336d10be7f2d1510cb65478f0ea3e0baea5d49ff962bcccdcf4396a0b3cfed0f1b8c5537b148f88f31e782f30be64807cad8900706b18a31cce9a743694b0abf94d6ff32789e870b3b70970bc2a01b69faea5a6dfc3514b4d6cf831dd715429cb3c9c3c9011422260233eab35f30dec5415fe06f9a22e5e4847cde93f61e896ebeec082ced1e65b7bf5dfe6f6dd064d2649580ae5ec6b09934167cdd0efc24150dee406c18dc4d6def110c74049a3f14c7d2b019606518ab91cba648915908d032c33cd3a6c07bfb908902c5a8bd55ed5fb25582659a9f4fb82aedba03c6946823b020ff8fad039772696c1b58a3434a5c53f5b6670943e90ccf49fb24d88929f467341cd68978082969dfc75ccdf161e1340bb3d66633b52703b2efd6cf769395fa892f5738cf5dee96afe27fe085bed54dd607bc0f0b3fe5fd5e83f1a18ed9e3457ac28bc6a49224c20f17d63fbc38f2d3e49af4f108407a9523e55fc1e89a2c221b0d15a993a3856a9f9618655555f7828734da3193ad2353c81a6f0720e90dbc62a8dcdd1e117b8f6addd574a6c483a5bebb06255e9614ff22ce4ac848de8ee8df47bd133fbd5f46bf9bf9a56e80d6e411cf2803186dad1a7cd9176ba85dff17e29471fb1c6f3a9304630e190406857e511c93711eca6a472f89005ddef430f0df953dcf5a3751bddaf39da32e25a87b1f41cc23f14b25ea9e0289785520696b0a82d6a23a19eb11ca32021c414ba83f0d4012933a4a962826e7185f21f440c8b08c1adf58aec9daee1c8e15e607239e819fc5dea80c697e800a1a18acd235789fb9dfee43f3e8a51ba190656ca8ee9dc7ed1cbfce26a0deb7563f52292f3f6bef6360095b1fa416afa01640ddbabbd3b8fc15223d50c0cdc80cb846947b80408764fab356051d2783e2a9e54917cfaab223c75dd8d5187841fbe93fc79bbc1d63ffffce68ae16c3b4ef3bd92d87bec21f2f958ab4f91535f10c50ef186e3a4d2a43b8060ac15b9ef21256e52123862563540c14d9d0904c20c70d2c5915e352b582f7ee0dfe3338658c1e7245b651428799705d9b76847e9fc8a872ef3aae9c978ca64e3f5f11dd7d49decaad5c299680e7478ddc9651d8578774431b46cc701601af616f9c7323ce76fcd1c6055f7d02652c9a2354ad21ebfd1df37d5254609e3d38666940a2a6dd21c59400bf444f8b297203243de4099b1c8640fb43849f160cdab42a52e0a107df5db400819f7587957f07d72cb498ae97aa6d1e67ae2900ff56f7378f742e04fcdedd2a72ef20aea340f9f65cff2bedc1362733170906a443a1964bdc59c245808014604e2fc9c9f23ecc590da6bedcb81c69ef8f369d69a0c9c663e0faccefde8bf848224166c59b49eb9a58f8fb38bdb42f6b33b5470378bfe21a980b1d78a8da4c32b4f380127bdd6a9c0c96f1b3ee4c0bbc69fa312e7a77560ad2eafdc97017ff9e51da30ee8e2acfaef091236c4c6cf66e2f43129d70744812d2eafdc97017ff9e51da30ee8e2acfaef091236c4c6cf66e2f43129d707448126981ddc905c11356d461b7ccc828dc1ac8e3c92cc9ba3619ee76f9150095a75304d64fd0d2d436f18e6881aae6b7d99bed17078b8f508f0cf4bb2dbd3e7f7871170c739f9d9ea4404bff4066c3ed34d6a52245965b485b766344a380f65e5d2800000000000000000000000000000000";

        let utx_bytes = hex::decode(fixed.replace(
            "GASPRICE",
            hex::encode((gas_price).to_le_bytes()).as_str(),
        ))
        .expect("decodable data");

        Transaction {
            inner: phoenix_core::Transaction::from_slice(&utx_bytes)
                .expect("should be valid"),
            gas_spent: None,
            err: None
        }
    }
}
