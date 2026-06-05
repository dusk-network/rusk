// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Phoenix-specific hardened HD derivation.
//!
//! Phoenix HD v1 derives account keys as:
//!
//! `seed -> master xsk -> m/32'/DUSK_COIN_TYPE'/account' -> (sk, vk, pk)`.
//!
//! The HD node secret is opaque and only exists above the existing Phoenix
//! `(a, b)`, `(a, B)`, `(A, B)` key model. The legacy seed+index derivation
//! remains available under [`super::legacy`] for migration.
//!
//! Phoenix HD v1 defines raw byte encodings only. Human-readable encodings are
//! intentionally out of scope until a concrete import/export flow exists.

use alloc::vec::Vec;
use core::fmt;
use core::ops::Range;
use core::str::FromStr;

use blake2b_simd::Params as Blake2bParams;
use dusk_bytes::{Error as BytesError, Serializable};
use dusk_core::transfer::phoenix::{
    PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    ViewKey as PhoenixViewKey,
};
use dusk_core::{JubJubExtended, JubJubScalar, GENERATOR_EXTENDED};
use ff::Field;
use zeroize::Zeroize;

use super::legacy;
use crate::Seed;

/// Phoenix HD purpose segment.
pub const PHOENIX_PURPOSE: u32 = 32;
/// Dusk SLIP-44 coin type used by Phoenix HD.
///
/// This matches the existing Moonlight/staking derivation coin type used by
/// Dusk wallet integrations. Phoenix HD v1 intentionally uses coin type `744`
/// even though SLIP-44 also contains the later `5995` Dusk Network entry. The
/// v1 path does not encode network-specific coin types; mainnet, testnet, and
/// devnet use the same account path shape.
pub const DUSK_COIN_TYPE: u32 = 744;
/// Hardened offset for HD children.
pub const HARDENED_OFFSET: u32 = 1 << 31;

/// Minimum seed length accepted by Phoenix HD.
pub const MIN_SEED_LEN: usize = 32;
/// Maximum seed length accepted by Phoenix HD.
pub const MAX_SEED_LEN: usize = 252;
/// Phoenix HD raw encoding version.
pub const PHOENIX_HD_VERSION: u8 = 1;
/// Raw kind value for Phoenix extended spending keys.
pub const PHOENIX_XSK_KIND: u8 = 1;
/// Raw kind value for Phoenix extended viewing keys.
pub const PHOENIX_XVK_KIND: u8 = 2;
/// Raw header byte for Phoenix extended spending keys.
///
/// The high nibble is the key kind and the low nibble is the format version.
pub const PHOENIX_XSK_HEADER: u8 = (PHOENIX_XSK_KIND << 4) | PHOENIX_HD_VERSION;
/// Raw header byte for Phoenix extended viewing keys.
///
/// The high nibble is the key kind and the low nibble is the format version.
pub const PHOENIX_XVK_HEADER: u8 = (PHOENIX_XVK_KIND << 4) | PHOENIX_HD_VERSION;

/// Master key generation `BLAKE2b` personalization.
pub const PHX_MKG_PERSONAL: &[u8] = b"DuskZIP32Phoenix";
/// Child key derivation `BLAKE2b` personalization.
pub const PHX_CKD_PERSONAL: &[u8] = b"DuskPhoenixCKD";
/// Expansion personalization for Phoenix scalar `a`.
pub const PHX_EXPAND_A_PERSONAL: &[u8] = b"DuskPhoenixExpA";
/// Expansion personalization for Phoenix scalar `b`.
pub const PHX_EXPAND_B_PERSONAL: &[u8] = b"DuskPhoenixExpB";
/// Parent-tag personalization derived from Phoenix public key bytes.
pub const PHX_PK_TAG_PERSONAL: &[u8] = b"DuskPhoenixPKTag";

/// Errors raised by Phoenix HD parsing and derivation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhoenixHdError {
    /// Seed input is too short or too large to encode.
    InvalidSeedLength,
    /// A derivation index is out of the non-hardened input range `[0, 2^31)`.
    InvalidChildIndex,
    /// The derivation depth overflowed `u8`.
    InvalidDepth,
    /// A derivation path string has invalid shape.
    InvalidPath,
    /// A derivation path component is malformed or not hardened.
    InvalidPathComponent,
    /// The derivation path purpose component does not match `32'`.
    InvalidPurpose,
    /// The derivation path coin type does not match the configured coin type.
    InvalidCoinType,
    /// Extended-key metadata is invalid for the requested type.
    InvalidMetadata,
}

impl fmt::Display for PhoenixHdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidSeedLength => "invalid seed length",
            Self::InvalidChildIndex => "invalid child index",
            Self::InvalidDepth => "invalid derivation depth",
            Self::InvalidPath => "invalid derivation path",
            Self::InvalidPathComponent => "invalid derivation path component",
            Self::InvalidPurpose => "invalid derivation purpose",
            Self::InvalidCoinType => "invalid derivation coin type",
            Self::InvalidMetadata => "invalid extended-key metadata",
        };

        write!(f, "{message}")
    }
}

/// Root Phoenix HD key.
#[derive(Clone, Zeroize)]
pub struct PhoenixMasterKey {
    xsk: PhoenixExtendedSpendingKey,
}

impl PhoenixMasterKey {
    fn new(xsk: PhoenixExtendedSpendingKey) -> Self {
        Self { xsk }
    }

    /// Creates a master key from a root extended spending key.
    ///
    /// # Errors
    ///
    /// Returns [`PhoenixHdError::InvalidMetadata`] if the extended spending key
    /// is not a root node.
    pub fn try_from_xsk(
        mut xsk: PhoenixExtendedSpendingKey,
    ) -> Result<Self, PhoenixHdError> {
        if !is_root_metadata(xsk.depth, xsk.parent_tag, xsk.child_number) {
            xsk.zeroize();
            return Err(PhoenixHdError::InvalidMetadata);
        }

        Ok(Self { xsk })
    }

    /// Returns the inner root extended spending key.
    #[must_use]
    pub fn as_xsk(&self) -> &PhoenixExtendedSpendingKey {
        &self.xsk
    }
}

/// Phoenix extended spending key.
///
/// `k` is an opaque node secret used to expand into Phoenix `(a, b)`.
#[derive(Clone, Zeroize)]
pub struct PhoenixExtendedSpendingKey {
    depth: u8,
    parent_tag: [u8; 4],
    child_number: u32,
    chain_code: [u8; 32],
    k: [u8; 32],
}

impl PhoenixExtendedSpendingKey {
    /// Raw serialized size for an extended spending key.
    pub const RAW_SIZE: usize = 74;

    fn new(
        depth: u8,
        parent_tag: [u8; 4],
        child_number: u32,
        chain_code: [u8; 32],
        k: [u8; 32],
    ) -> Self {
        Self {
            depth,
            parent_tag,
            child_number,
            chain_code,
            k,
        }
    }

    /// Returns the derivation depth.
    #[must_use]
    pub const fn depth(&self) -> u8 {
        self.depth
    }

    /// Returns the parent tag metadata.
    ///
    /// The tag is only a lookup hint. It is not a unique identifier.
    #[must_use]
    pub const fn parent_tag(&self) -> [u8; 4] {
        self.parent_tag
    }

    /// Returns the serialized child number metadata.
    #[must_use]
    pub const fn child_number(&self) -> u32 {
        self.child_number
    }

    /// Returns the chain code.
    #[must_use]
    pub const fn chain_code(&self) -> [u8; 32] {
        self.chain_code
    }

    /// Returns the corresponding Phoenix extended viewing key.
    #[must_use]
    pub fn to_xvk(&self) -> PhoenixExtendedViewingKey {
        phoenix_view_key(self)
    }

    /// Returns the corresponding Phoenix secret key `(a, b)`.
    #[must_use]
    pub fn to_secret_key(&self) -> PhoenixSecretKey {
        phoenix_secret_key(self)
    }

    /// Returns the corresponding Phoenix public key `(A, B)`.
    #[must_use]
    pub fn to_public_key(&self) -> PhoenixPublicKey {
        phoenix_public_key(self)
    }
}

impl Serializable<74> for PhoenixExtendedSpendingKey {
    type Error = BytesError;

    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0] = PHOENIX_XSK_HEADER;
        buf[1] = self.depth;
        buf[2..6].copy_from_slice(&self.parent_tag);
        buf[6..10].copy_from_slice(&self.child_number.to_le_bytes());
        buf[10..42].copy_from_slice(&self.chain_code);
        buf[42..74].copy_from_slice(&self.k);

        buf
    }

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        if buf[0] != PHOENIX_XSK_HEADER {
            return Err(BytesError::InvalidData);
        }

        let depth = buf[1];
        let mut parent_tag = [0u8; 4];
        parent_tag.copy_from_slice(&buf[2..6]);

        let child_number =
            u32::from_le_bytes(buf[6..10].try_into().expect("slice length"));
        validate_metadata(depth, parent_tag, child_number)?;

        let mut chain_code = [0u8; 32];
        chain_code.copy_from_slice(&buf[10..42]);

        let mut k = [0u8; 32];
        k.copy_from_slice(&buf[42..74]);

        Ok(Self::new(depth, parent_tag, child_number, chain_code, k))
    }
}

/// Phoenix extended viewing key.
///
/// This key is sensitive. It contains the Phoenix view key, which includes
/// secret scalar `a`. Phoenix HD v1 does not derive child keys from it.
#[derive(Clone)]
pub struct PhoenixExtendedViewingKey {
    depth: u8,
    parent_tag: [u8; 4],
    child_number: u32,
    view_key: PhoenixViewKey,
}

impl Zeroize for PhoenixExtendedViewingKey {
    fn zeroize(&mut self) {
        self.depth.zeroize();
        self.parent_tag.zeroize();
        self.child_number.zeroize();
        self.view_key = zero_view_key();
    }
}

impl PhoenixExtendedViewingKey {
    /// Raw serialized size for an extended viewing key.
    pub const RAW_SIZE: usize = 74;

    fn new(
        depth: u8,
        parent_tag: [u8; 4],
        child_number: u32,
        view_key: PhoenixViewKey,
    ) -> Self {
        Self {
            depth,
            parent_tag,
            child_number,
            view_key,
        }
    }

    /// Returns the derivation depth.
    #[must_use]
    pub const fn depth(&self) -> u8 {
        self.depth
    }

    /// Returns the parent tag metadata.
    ///
    /// The tag is only a lookup hint. It is not a unique identifier.
    #[must_use]
    pub const fn parent_tag(&self) -> [u8; 4] {
        self.parent_tag
    }

    /// Returns the serialized child number metadata.
    #[must_use]
    pub const fn child_number(&self) -> u32 {
        self.child_number
    }

    /// Returns the Phoenix view key.
    #[must_use]
    pub const fn view_key(&self) -> PhoenixViewKey {
        self.view_key
    }

    /// Returns the Phoenix public key reconstructed from the view key.
    #[must_use]
    pub fn public_key(&self) -> PhoenixPublicKey {
        PhoenixPublicKey::from(&self.view_key)
    }
}

impl Serializable<74> for PhoenixExtendedViewingKey {
    type Error = BytesError;

    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0] = PHOENIX_XVK_HEADER;
        buf[1] = self.depth;
        buf[2..6].copy_from_slice(&self.parent_tag);
        buf[6..10].copy_from_slice(&self.child_number.to_le_bytes());
        buf[10..].copy_from_slice(&self.view_key.to_bytes());

        buf
    }

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        if buf[0] != PHOENIX_XVK_HEADER {
            return Err(BytesError::InvalidData);
        }

        let depth = buf[1];
        let mut parent_tag = [0u8; 4];
        parent_tag.copy_from_slice(&buf[2..6]);

        let child_number =
            u32::from_le_bytes(buf[6..10].try_into().expect("slice length"));
        validate_metadata(depth, parent_tag, child_number)?;

        let mut vk_bytes = [0u8; 64];
        vk_bytes.copy_from_slice(&buf[10..]);
        let view_key = PhoenixViewKey::from_bytes(&vk_bytes)?;

        Ok(Self::new(depth, parent_tag, child_number, view_key))
    }
}

/// Phoenix account derivation path.
///
/// The supported format is `m / 32' / DUSK_COIN_TYPE' / account'`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Zeroize)]
pub struct PhoenixDerivationPath {
    purpose: u32,
    coin_type: u32,
    account: u32,
}

impl PhoenixDerivationPath {
    /// Creates a path for the configured Dusk coin type.
    ///
    /// # Errors
    ///
    /// Returns [`PhoenixHdError::InvalidChildIndex`] if `account` is outside
    /// the non-hardened input range `[0, 2^31)`.
    pub fn new(account: u32) -> Result<Self, PhoenixHdError> {
        Self::with_coin_type(DUSK_COIN_TYPE, account)
    }

    /// Creates a path using a custom coin type.
    ///
    /// # Errors
    ///
    /// Returns [`PhoenixHdError::InvalidChildIndex`] if `coin_type` or
    /// `account` is outside the non-hardened input range `[0, 2^31)`.
    pub fn with_coin_type(
        coin_type: u32,
        account: u32,
    ) -> Result<Self, PhoenixHdError> {
        if coin_type >= HARDENED_OFFSET || account >= HARDENED_OFFSET {
            return Err(PhoenixHdError::InvalidChildIndex);
        }

        Ok(Self {
            purpose: PHOENIX_PURPOSE,
            coin_type,
            account,
        })
    }

    /// Returns the purpose segment.
    #[must_use]
    pub const fn purpose(&self) -> u32 {
        self.purpose
    }

    /// Returns the coin type segment.
    #[must_use]
    pub const fn coin_type(&self) -> u32 {
        self.coin_type
    }

    /// Returns the account segment.
    #[must_use]
    pub const fn account(&self) -> u32 {
        self.account
    }

    /// Returns all path segments in derivation order.
    #[must_use]
    pub const fn segments(&self) -> [u32; 3] {
        [self.purpose, self.coin_type, self.account]
    }
}

impl fmt::Display for PhoenixDerivationPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "m/{}'/{}'/{}'",
            self.purpose, self.coin_type, self.account
        )
    }
}

impl FromStr for PhoenixDerivationPath {
    type Err = PhoenixHdError;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let mut parts = path.split('/');
        if parts.next() != Some("m") {
            return Err(PhoenixHdError::InvalidPath);
        }

        let purpose = parse_hardened_segment(
            parts.next().ok_or(PhoenixHdError::InvalidPath)?,
        )?;
        let coin_type = parse_hardened_segment(
            parts.next().ok_or(PhoenixHdError::InvalidPath)?,
        )?;
        let account = parse_hardened_segment(
            parts.next().ok_or(PhoenixHdError::InvalidPath)?,
        )?;

        if parts.next().is_some() {
            return Err(PhoenixHdError::InvalidPath);
        }

        if purpose != PHOENIX_PURPOSE {
            return Err(PhoenixHdError::InvalidPurpose);
        }
        if coin_type != DUSK_COIN_TYPE {
            return Err(PhoenixHdError::InvalidCoinType);
        }

        Self::new(account)
    }
}

/// Derived Phoenix account keys.
#[derive(Clone)]
pub struct PhoenixAccount {
    /// Derivation path.
    pub path: PhoenixDerivationPath,
    /// Phoenix secret key.
    pub sk: PhoenixSecretKey,
    /// Phoenix view key.
    pub vk: PhoenixViewKey,
    /// Phoenix public key.
    pub pk: PhoenixPublicKey,
}

impl Zeroize for PhoenixAccount {
    fn zeroize(&mut self) {
        self.path.zeroize();
        self.sk.zeroize();
        self.vk = zero_view_key();
        self.pk = zero_public_key();
    }
}

/// Legacy Phoenix account keys for a profile.
#[derive(Clone)]
pub struct PhoenixLegacyAccount {
    /// Legacy Phoenix secret key.
    pub sk: PhoenixSecretKey,
    /// Legacy Phoenix view key.
    pub vk: PhoenixViewKey,
    /// Legacy Phoenix public key.
    pub pk: PhoenixPublicKey,
}

impl Zeroize for PhoenixLegacyAccount {
    fn zeroize(&mut self) {
        self.sk.zeroize();
        self.vk = zero_view_key();
        self.pk = zero_public_key();
    }
}

/// Legacy + HD Phoenix account mapping for migration.
#[derive(Clone)]
pub struct PhoenixMigrationAccount {
    /// User-visible profile index.
    pub profile: u8,
    /// Legacy account keys derived using `legacy` APIs.
    pub legacy: PhoenixLegacyAccount,
    /// HD account keys.
    pub hd: PhoenixAccount,
}

impl Zeroize for PhoenixMigrationAccount {
    fn zeroize(&mut self) {
        self.profile.zeroize();
        self.legacy.zeroize();
        self.hd.zeroize();
    }
}

/// Derives the Phoenix master key from seed bytes.
///
/// Phoenix HD v1 accepts 32 to 252 bytes of canonical seed material and encodes
/// it as `u32_le(len(seed)) || seed` before master-key generation.
///
/// # Errors
///
/// Returns [`PhoenixHdError::InvalidSeedLength`] if `seed` is not between
/// [`MIN_SEED_LEN`] and [`MAX_SEED_LEN`] bytes.
pub fn phoenix_master_from_seed(
    seed: &[u8],
) -> Result<PhoenixMasterKey, PhoenixHdError> {
    if !(MIN_SEED_LEN..=MAX_SEED_LEN).contains(&seed.len()) {
        return Err(PhoenixHdError::InvalidSeedLength);
    }

    let seed_len = u32::try_from(seed.len())
        .map_err(|_| PhoenixHdError::InvalidSeedLength)?;
    let mut ikm = Vec::with_capacity(4 + seed.len());
    ikm.extend_from_slice(&seed_len.to_le_bytes());
    ikm.extend_from_slice(seed);

    let mut i = blake2b_personal::<64>(PHX_MKG_PERSONAL, &ikm);
    let mut k = [0u8; 32];
    let mut c = [0u8; 32];
    k.copy_from_slice(&i[..32]);
    c.copy_from_slice(&i[32..]);
    ikm.zeroize();
    i.zeroize();

    let xsk = PhoenixExtendedSpendingKey::new(0, [0u8; 4], 0, c, k);
    k.zeroize();
    c.zeroize();

    Ok(PhoenixMasterKey::new(xsk))
}

/// Derives the account extended spending key from a master key.
///
/// Path: `m / 32' / DUSK_COIN_TYPE' / account'`
///
/// # Errors
///
/// Returns [`PhoenixHdError::InvalidChildIndex`] if `account` is outside the
/// non-hardened input range `[0, 2^31)`, or [`PhoenixHdError::InvalidDepth`] if
/// child derivation would overflow the encoded depth.
pub fn phoenix_account_xsk(
    master: &PhoenixMasterKey,
    account: u32,
) -> Result<PhoenixExtendedSpendingKey, PhoenixHdError> {
    let path = PhoenixDerivationPath::new(account)?;
    derive_path(master, &path)
}

/// Derives Phoenix account keys from a master key.
///
/// # Errors
///
/// Returns [`PhoenixHdError::InvalidChildIndex`] if `account` is outside the
/// non-hardened input range `[0, 2^31)`, or [`PhoenixHdError::InvalidDepth`] if
/// child derivation would overflow the encoded depth.
pub fn phoenix_account(
    master: &PhoenixMasterKey,
    account: u32,
) -> Result<PhoenixAccount, PhoenixHdError> {
    let path = PhoenixDerivationPath::new(account)?;
    let mut xsk = derive_path(master, &path)?;
    let mut scalars = expand_scalars(&xsk.k);
    let sk = scalars.secret_key();
    let vk = scalars.view_key();
    let pk = scalars.public_key();
    scalars.zeroize();
    xsk.zeroize();

    Ok(PhoenixAccount { path, sk, vk, pk })
}

/// Expands an extended spending key into a Phoenix secret key.
#[must_use]
pub fn phoenix_secret_key(
    xsk: &PhoenixExtendedSpendingKey,
) -> PhoenixSecretKey {
    let mut scalars = expand_scalars(&xsk.k);
    let sk = scalars.secret_key();
    scalars.zeroize();

    sk
}

/// Expands an extended spending key into an extended viewing key.
#[must_use]
pub fn phoenix_view_key(
    xsk: &PhoenixExtendedSpendingKey,
) -> PhoenixExtendedViewingKey {
    let mut scalars = expand_scalars(&xsk.k);
    let xvk = PhoenixExtendedViewingKey::new(
        xsk.depth,
        xsk.parent_tag,
        xsk.child_number,
        scalars.view_key(),
    );
    scalars.zeroize();

    xvk
}

/// Expands an extended spending key into a Phoenix public key.
#[must_use]
pub fn phoenix_public_key(
    xsk: &PhoenixExtendedSpendingKey,
) -> PhoenixPublicKey {
    let mut scalars = expand_scalars(&xsk.k);
    let pk = scalars.public_key();
    scalars.zeroize();

    pk
}

/// Reconstructs the Phoenix public key from an extended viewing key.
#[must_use]
pub fn phoenix_public_key_from_xvk(
    xvk: &PhoenixExtendedViewingKey,
) -> PhoenixPublicKey {
    PhoenixPublicKey::from(&xvk.view_key)
}

/// Returns the public-key tag used in child metadata.
///
/// The tag is only a lookup hint. It is not a unique identifier.
#[must_use]
pub fn phoenix_public_key_tag(pk: &PhoenixPublicKey) -> [u8; 4] {
    let bytes = pk.to_bytes();
    let fingerprint = blake2b_personal::<32>(PHX_PK_TAG_PERSONAL, &bytes);

    let mut tag = [0u8; 4];
    tag.copy_from_slice(&fingerprint[..4]);
    tag
}

/// Derives a legacy Phoenix account for migration.
#[must_use]
pub fn phoenix_legacy_account(
    seed: &Seed,
    profile: u8,
) -> PhoenixLegacyAccount {
    let sk = legacy::derive_phoenix_sk(seed, profile);
    let vk = PhoenixViewKey::from(&sk);
    let pk = PhoenixPublicKey::from(&sk);

    PhoenixLegacyAccount { sk, vk, pk }
}

/// Derives legacy + HD account keys for one profile.
///
/// # Errors
///
/// Returns [`PhoenixHdError::InvalidSeedLength`] if the seed policy rejects the
/// seed, or any account-derivation error raised for the HD profile path.
pub fn phoenix_migration_account(
    seed: &Seed,
    profile: u8,
) -> Result<PhoenixMigrationAccount, PhoenixHdError> {
    let mut master = phoenix_master_from_seed(seed)?;
    let hd = phoenix_account(&master, u32::from(profile));
    master.zeroize();

    let hd = hd?;
    let legacy = phoenix_legacy_account(seed, profile);

    Ok(PhoenixMigrationAccount {
        profile,
        legacy,
        hd,
    })
}

/// Derives legacy + HD account bundles for a profile range.
///
/// # Errors
///
/// Returns [`PhoenixHdError::InvalidSeedLength`] if the seed policy rejects the
/// seed, or any account-derivation error raised for an HD profile path.
pub fn phoenix_dual_scan_accounts(
    seed: &Seed,
    profiles: Range<u8>,
) -> Result<Vec<PhoenixMigrationAccount>, PhoenixHdError> {
    let mut master = phoenix_master_from_seed(seed)?;

    let accounts = profiles
        .map(|profile| {
            let hd = phoenix_account(&master, u32::from(profile))?;
            let legacy = phoenix_legacy_account(seed, profile);

            Ok(PhoenixMigrationAccount {
                profile,
                legacy,
                hd,
            })
        })
        .collect();
    master.zeroize();

    accounts
}

fn derive_path(
    master: &PhoenixMasterKey,
    path: &PhoenixDerivationPath,
) -> Result<PhoenixExtendedSpendingKey, PhoenixHdError> {
    let mut node = master.as_xsk().clone();
    for child in path.segments() {
        let next = derive_hardened_node(&node, child);
        node.zeroize();
        node = next?;
    }
    Ok(node)
}

fn derive_hardened_node(
    parent: &PhoenixExtendedSpendingKey,
    child: u32,
) -> Result<PhoenixExtendedSpendingKey, PhoenixHdError> {
    if child >= HARDENED_OFFSET {
        return Err(PhoenixHdError::InvalidChildIndex);
    }

    let depth = parent
        .depth
        .checked_add(1)
        .ok_or(PhoenixHdError::InvalidDepth)?;
    let child_hardened = child + HARDENED_OFFSET;

    let mut msg = Vec::with_capacity(
        parent.chain_code.len() + parent.k.len() + core::mem::size_of::<u32>(),
    );
    msg.extend_from_slice(&parent.chain_code);
    msg.extend_from_slice(&parent.k);
    msg.extend_from_slice(&child_hardened.to_le_bytes());

    let mut i = blake2b_personal::<64>(PHX_CKD_PERSONAL, &msg);
    let mut k = [0u8; 32];
    let mut c = [0u8; 32];
    k.copy_from_slice(&i[..32]);
    c.copy_from_slice(&i[32..]);
    msg.zeroize();
    i.zeroize();

    let parent_pk = phoenix_public_key(parent);
    let parent_tag = phoenix_public_key_tag(&parent_pk);

    let child = PhoenixExtendedSpendingKey::new(
        depth,
        parent_tag,
        child_hardened,
        c,
        k,
    );
    k.zeroize();
    c.zeroize();

    Ok(child)
}

fn validate_metadata(
    depth: u8,
    parent_tag: [u8; 4],
    child_number: u32,
) -> Result<(), BytesError> {
    if depth == 0 {
        if !is_root_metadata(depth, parent_tag, child_number) {
            return Err(BytesError::InvalidData);
        }
    } else if child_number < HARDENED_OFFSET {
        return Err(BytesError::InvalidData);
    }

    Ok(())
}

fn is_root_metadata(depth: u8, parent_tag: [u8; 4], child_number: u32) -> bool {
    depth == 0 && parent_tag == [0u8; 4] && child_number == 0
}

fn parse_hardened_segment(segment: &str) -> Result<u32, PhoenixHdError> {
    if segment.len() < 2 || !segment.ends_with('\'') {
        return Err(PhoenixHdError::InvalidPathComponent);
    }

    let value = &segment[..segment.len() - 1];
    let parsed = value
        .parse::<u32>()
        .map_err(|_| PhoenixHdError::InvalidPathComponent)?;
    if parsed >= HARDENED_OFFSET {
        return Err(PhoenixHdError::InvalidChildIndex);
    }

    Ok(parsed)
}

fn blake2b_personal<const N: usize>(personal: &[u8], data: &[u8]) -> [u8; N] {
    let digest = Blake2bParams::new()
        .hash_length(N)
        .personal(personal)
        .hash(data);

    let mut out = [0u8; N];
    out.copy_from_slice(digest.as_bytes());
    out
}

fn expand_scalars(k: &[u8; 32]) -> PhoenixScalars {
    let a = expand_nonzero_scalar(PHX_EXPAND_A_PERSONAL, k);
    let b = expand_nonzero_scalar(PHX_EXPAND_B_PERSONAL, k);

    PhoenixScalars { a, b }
}

fn expand_nonzero_scalar(personal: &[u8], k: &[u8; 32]) -> JubJubScalar {
    let mut counter = 0u32;
    loop {
        let mut data = [0u8; 36];
        data[..32].copy_from_slice(k);
        data[32..].copy_from_slice(&counter.to_le_bytes());

        let mut wide = blake2b_personal::<64>(personal, &data);
        let scalar = nonzero_scalar_from_wide(&wide);
        data.zeroize();
        wide.zeroize();

        if let Some(scalar) = scalar {
            return scalar;
        }

        counter = counter
            .checked_add(1)
            .expect("u32 counter space should not be exhausted");
    }
}

fn nonzero_scalar_from_wide(wide: &[u8; 64]) -> Option<JubJubScalar> {
    let scalar = JubJubScalar::from_bytes_wide(wide);
    (!bool::from(scalar.is_zero())).then_some(scalar)
}

#[derive(Zeroize)]
struct PhoenixScalars {
    a: JubJubScalar,
    b: JubJubScalar,
}

impl PhoenixScalars {
    fn secret_key(&self) -> PhoenixSecretKey {
        PhoenixSecretKey::new(self.a, self.b)
    }

    fn view_key(&self) -> PhoenixViewKey {
        let b = GENERATOR_EXTENDED * self.b;

        PhoenixViewKey::new(self.a, b)
    }

    fn public_key(&self) -> PhoenixPublicKey {
        let a = GENERATOR_EXTENDED * self.a;
        let b = GENERATOR_EXTENDED * self.b;

        PhoenixPublicKey::new(a, b)
    }
}

fn zero_view_key() -> PhoenixViewKey {
    PhoenixViewKey::new(JubJubScalar::zero(), JubJubExtended::identity())
}

fn zero_public_key() -> PhoenixPublicKey {
    PhoenixPublicKey::new(
        JubJubExtended::identity(),
        JubJubExtended::identity(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: [u8; 64] = [0u8; 64];

    #[test]
    fn child_parent_tag_matches_parent_public_key() {
        let master = phoenix_master_from_seed(&SEED).expect("master key");
        let parent = derive_hardened_node(master.as_xsk(), PHOENIX_PURPOSE)
            .expect("purpose node");
        let child =
            derive_hardened_node(&parent, DUSK_COIN_TYPE).expect("coin node");

        assert_eq!(
            child.parent_tag(),
            phoenix_public_key_tag(&phoenix_public_key(&parent))
        );
    }

    #[test]
    fn zero_scalar_wide_reduction_is_rejected() {
        assert!(nonzero_scalar_from_wide(&[0u8; 64]).is_none());

        let mut wide = [0u8; 64];
        wide[0] = 1;
        assert!(nonzero_scalar_from_wide(&wide).is_some());
    }
}
