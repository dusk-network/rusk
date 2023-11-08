// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{bls, Serializable};
use rusk_abi::hash::Hasher;
use sha3::Digest;
use std::io::{self, Read, Write};

#[cfg(any(feature = "faker", test))]
use fake::{Dummy, Fake, Faker};

pub type Seed = Signature;
pub type Hash = [u8; 32];

#[derive(Default, Debug, Clone)]
pub struct Block {
    header: Header,
    txs: Vec<Transaction>,
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
    pub event_hash: Hash,
    pub generator_bls_pubkey: bls::PublicKeyBytes,
    pub txroot: Hash,
    pub gas_limit: u64,
    pub iteration: u8,
    pub prev_block_cert: Certificate,

    // Block hash
    pub hash: Hash,

    // Non-hashable fields
    pub cert: Certificate,
}

impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let timestamp = chrono::NaiveDateTime::from_timestamp_opt(
            self.timestamp,
            0,
        )
        .map_or_else(
            || "unknown".to_owned(),
            |v| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
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
            .field("prev_block_hash", &to_str(&self.prev_block_hash))
            .field("seed", &to_str(&self.seed.inner()))
            .field("state_hash", &to_str(&self.state_hash))
            .field("event_hash", &to_str(&self.event_hash))
            .field("gen_bls_pubkey", &to_str(self.generator_bls_pubkey.inner()))
            .field("gas_limit", &self.gas_limit)
            .field("hash", &to_str(&self.hash))
            .field("cert", &self.cert)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: u32,
    pub r#type: u32,
    pub inner: phoenix_core::Transaction,
}

impl From<phoenix_core::Transaction> for Transaction {
    fn from(value: phoenix_core::Transaction) -> Self {
        Self {
            inner: value,
            r#type: 1,
            version: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpentTransaction {
    pub inner: Transaction,
    pub block_height: u64,
    pub gas_spent: u64,
    pub err: Option<String>,
}

impl Transaction {
    pub fn hash(&self) -> [u8; 32] {
        Hasher::digest(self.inner.to_hash_input_bytes()).to_bytes()
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
            Self::write_var_bytes(w, &self.seed.inner()[..])?;
        }

        w.write_all(&self.state_hash[..])?;
        w.write_all(&self.event_hash[..])?;
        w.write_all(&self.generator_bls_pubkey.inner()[..])?;
        w.write_all(&self.txroot[..])?;
        w.write_all(&self.gas_limit.to_le_bytes())?;
        w.write_all(&self.iteration.to_le_bytes())?;
        self.prev_block_cert.write(w)?;

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

        let value = Self::read_var_bytes(r)?;
        let seed: [u8; 48] = value
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut state_hash = [0u8; 32];
        r.read_exact(&mut state_hash[..])?;

        let mut event_hash = [0u8; 32];
        r.read_exact(&mut event_hash[..])?;

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

        let prev_block_cert = Certificate::read(r)?;

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
            event_hash,
            txroot,
            hash: [0; 32],
            cert: Default::default(),
            prev_block_cert,
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

    fn calculate_hash(&mut self) -> io::Result<()> {
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

    pub fn set_certificate(&mut self, cert: Certificate) {
        self.header.cert = cert;
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct StepVotes {
    pub bitset: u64,
    pub aggregate_signature: Signature,
}

impl StepVotes {
    pub fn new(aggregate_signature: [u8; 48], bitset: u64) -> StepVotes {
        StepVotes {
            bitset,
            aggregate_signature: Signature(aggregate_signature),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bitset == 0 || self.aggregate_signature.is_zeroed()
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
            .field("signature", &to_str(&self.0))
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
        self.r#type == other.r#type
            && self.version == other.version
            && self.hash() == other.hash()
    }
}

impl Eq for Transaction {}

impl PartialEq<Self> for SpentTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.gas_spent == other.gas_spent
    }
}

impl Eq for SpentTransaction {}

/// Encode a byte array into a shortened HEX representation.
pub fn to_str<const N: usize>(bytes: &[u8; N]) -> String {
    let e = hex::encode(bytes);
    if e.len() != bytes.len() * 2 {
        return String::from("invalid hex");
    }

    const OFFSET: usize = 16;
    let (first, last) = e.split_at(OFFSET);
    let (_, second) = last.split_at(e.len() - 2 * OFFSET);

    first.to_owned() + "..." + second
}

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

    impl<T> Dummy<T> for SpentTransaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            let tx = gen_dummy_tx(1_000_000);
            SpentTransaction {
                inner: tx,
                block_height: 0,
                gas_spent: 3,
                err: Some("error".to_string()),
            }
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
        let fixed = "31858df61df8a7208b64540f1dafffcce0ec7613f38d6147ca4393c101d45e600200000000000000f1afb77f1551f559f1226f88b2efbbc37c811218cc50531f0a8a704c98c35a32e1b905bb45a09cbca4e003a9f9d2eb6c16adf6fa49ca8afd8dfdfca449f354700100000000000000017918bbfc50c9a07b4abf98e605085f543cca07ca802b94afe1d992995543335a2d6307d9dc46929cb072d09bc790b6214671fc55df8e8d7af1bb3ef33529d31dce2982d5fef7fd0f7e36ba3e88d119fb807a546f3e273d09380dbdf48d0c57f0513c03a5c1e98e40493acde840d384c965bfccdb26af78aec81333fe24028fc2ffffffffffffffffe2f34d13ef74e08d966ced4a75ea36af891247031f848b003755dff9e8060b01104e6237b437d401ba7ad37a2435fbcb39ad17e3308a0a80c97acbe336e68851d034589b1859998aeced6f060874daf17299ae5cba212020f6bb80701afb1f410065cd1d00000000GASPRICEd870bd7510b7d51655ce1122829ec083b7d0a5a4db2ffe16589b3eabaec51d0f7a600e3dae06a4f740336cedd2b5bbeaf50428fbee075bd29cef630bab5f86a9001004000000000000a0f630636f64629f0a4422b00d5dbccb0405c4124603b4f00b29ef1ab49a617bacda7c6ea142a7ca80b1817f072a0c8491a6966f8b648c6e552b4afaa4fc5d1867448d1e3a1a360cccb68d54f100e47ab0a98464e8e8bbcca13c057ea89bb0f1b1aedd1ed10e18b5661f2dd029ecb7060110cd784837675c5588cbc5721b9075eb9b6c6a1dacff8fd9ae5e2587994f0e8b74ecc9cd9254916dcb7518d807d452ba551a5590d3045283739f8f954d3550ae08717f717bcff71a8c87d3725135abb0e1cf62b16d3e0185d589f2c84f04b577ec154ab770719eec1b3e935736021a49dbfdbd0fc76e12d33d516e3ab187b680b5a42b6f9d8104699a0e5d440db122698bbc0e7d7907d32d73664b88a4efe650bcd2cd8e17b6e1060fb99e0ba7cea790bae9e87f920bd37bb5d95946e679150560659f037cc148780745b21a8f8f726a82ec597f8195622fda20700cc38bdeb1d64eb47a240c3c72b4e1752677e39157764231ee2667ab36ccdbfba42457f8aa78190e9f349fdb0f39419a0118585b830e1f5384814e9656b8979aa59833641c241d35c05dc76d7e364aee2450106ab83406673fc18e931b92e5ccdb9c147b867649519adf577c562d87284c4674ee2b4300dc24d0f1a6dc1181b28b14322e7102cb0bf4959f2d49c55f76b438bb3b98b1ae81660da05395dd2780c2b7a57f91dfea8ab8e0ef39c6d3aa90b6a9c796dc7a5442582066d7eb35d61e41e26ad02c5bec835f0e56f87369a55eb8aa9267638b4cffbf8595a67158f55c8dc85e4c607c114f2d51f5bbd0424906bd3a49b3979ccc1fd3266c11b4f6fee01452a215c2537200e2f96527fbfb2a2525cf4a856a3d7ad224f90e77ebd1564d83a75c34e4bfbb3a2e9112af6d5d684c693079671817d04798e47a891313e74401893d3cbed23ffcba13c62a9815a2207d3b4791a77bcd52326a8acd57918a371552bb46e486f1a6217e10ae74f5f8e4c1984c28ef8ac960abeb001c26282439eae48249cce5272acaba1be1ce770cd90c68efbda84dd3ccf2f2ce51d9b7351703d7cb6d928f27d0a662c26e54a6342d509e877b8355a50c2fb23f8f680c063a49d6ab31a2b34a17038141b04c19221df2f229ce7ca70b837ebf0c3a030527fc7037172d0ddc595e1aaba4774c1746a83dff4867b5986c5afe399c93ead34e37aaa79e48def89950118f5ab0c7d66074dd680fbc7146c977135a6aef55617befeba25f269b606a21f8880ce2c30e0ce8b898643c08871773ac1e41f62d86c24060b1072418275ded45bb819939ca45d681878ec16257902ce4d4aebc709027d77f2c8c622ce7f1a7f79a2d5efd460266a059ad242ff7b64947f44d1b9bd8d29a633b9223fb75ce14deb158c81e9a1703395531d173bdd88b8df90bee088618e12b875e42f45ff5cc7159b136e05b2426628b0dafa076e5dcc223a37cf5f0df7e75607f040101000000000000000000000000000000000000000000000000000000000000000400000000000000726f6f747aaa96c657582d963de258f184da3d7c95042920db92a2b2591c181466ea5a5311b5ef84f135b3d567120eaae941f35f8d5d62a8f42c8c9dacd2c78cc6f02e558d0d18fc4e78997967851fef237755ee6ee490036925c7fb100eefdbec1c033b8332252314a8fd0c05a01f9a8b87372381ee179818e8ffff36dc94a411c0d75e9ded4fcdf533fd58fc80c8dae586d783ed32168744d0f952dee4f02ce4e25f552327ac75c4aba87513dc54e4a5c2e2e36835828113ef92fea59dbf2457575903bf3156016acac7a44a1d6b546f4199bba7b24966c739ddc433a521ee5a776148349a0a53a6cd16d1cd20e13120bd404fd23a752a8f3facd53b78d360d3f3a03e57bf726eed5f1d8d4c530cd187919796fbb4cdc4dceb809f3839bcb453ae7022ae10a0f7ee7f4355e9d74fbf55d3221bb0beb1bf9ae1f6927e9d3cce9f24581640d55c7bf2cd4b6f130ef439a20b990223bb9022446b7e02e502619b4dd3f80d31e6ca8a4c8785453c9a4886d5e9159bcd1974030e70d1cdd008f3408845871c4cbfb0554963f5f47b91a81a0d1f7370be7c072d77a9e03489921981f503ec2f29a07829b2b4f9f285e2ff0d50378d59b12ad2fb8386001e561c3c3c5e66df31256fac5c58b056a49ffe0c23e8e583944e0252529b1c8438fbd22be0d89d5838bc023097bb353188a830dc1a280a0d36ad045d0691354f4a3cdf06724101be6bc23a3f90ad62b7eb699b215644326e61c546068bb46871cc5007e81aa117c1538a9297e803ff05b1f631d15791d5f3e179718a3a752649ef7c13f4e03f0390117a7e8542930b0d81da721a28e246a451c995bcc2f6a1961b56c6e0c030563068b32b6a97b404c884d9430e60d070354accd59c9e4e01f816c149898241f7f42e8724c0af7e5e84b28c9d830add2ab94c78dc474e06ca84d444267d37c86697615396cd4c74e95815fafd25e0f96ea952a9d2de0fbb646e83bf25147ed597df42";

        let utx_bytes = hex::decode(fixed.replace(
            "GASPRICE",
            hex::encode((gas_price).to_le_bytes()).as_str(),
        ))
        .expect("decodable data");

        let inner = phoenix_core::Transaction::from_slice(&utx_bytes)
            .expect("should be valid");
        inner.into()
    }
}
