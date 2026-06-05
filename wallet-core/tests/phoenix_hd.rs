// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::str::FromStr;

use dusk_bytes::Serializable;
use dusk_core::transfer::phoenix::{
    PublicKey as PhoenixPublicKey, ViewKey as PhoenixViewKey,
};
use dusk_core::JubJubScalar;
use dusk_wallet_core::keys::phoenix_hd::{
    phoenix_account, phoenix_account_xsk, phoenix_master_from_seed,
    phoenix_migration_account, phoenix_public_key_from_xvk, phoenix_view_key,
    PhoenixDerivationPath, PhoenixExtendedSpendingKey,
    PhoenixExtendedViewingKey, PhoenixHdError, PhoenixMasterKey,
    HARDENED_OFFSET, MAX_SEED_LEN, PHOENIX_HD_VERSION, PHOENIX_XSK_KIND,
    PHOENIX_XVK_KIND,
};
use dusk_wallet_core::keys::{legacy, phoenix_dual_scan_accounts};
use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

const SEED: [u8; 64] = [0u8; 64];
const NONZERO_SEED: [u8; 64] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
    40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58,
    59, 60, 61, 62, 63,
];
const MASTER_XSK_HEX: &str = "1100000000000000000056e41ea399aaf4b6b756cde9c443f817362083d79ed59e9441fa507cbec43d5367ac748ca8c1308193f345e7e82418f10443066400aeec3d0f75716409a1bf3c";
const ACC0_XSK_HEX: &str = "11039b762c86000000809c068503e96b66090907be856a53ec086416deaa6cd2ff2bd0dc9ee590f5e41c10bc87953587b4d9344e79188e9cc3737b3f71f3c4fc887f487d94c30803d736";
const ACC0_XVK_HEX: &str = "21039b762c86000000804ac05771047e0d32cae20bdee8f404c1e7fab01337a92ceffdc607ad382e5d0a4a6adddfdabf10016958bd1d67b90a5e5abde8d359fd3c7986e8a2a6c8b7f897";
const ACC0_SK_HEX: &str = "4ac05771047e0d32cae20bdee8f404c1e7fab01337a92ceffdc607ad382e5d0ab25993a50ad204ee81bee0f3bea0e4fac63f220fa75d2846e52bd08516fbc801";
const ACC0_VK_HEX: &str = "4ac05771047e0d32cae20bdee8f404c1e7fab01337a92ceffdc607ad382e5d0a4a6adddfdabf10016958bd1d67b90a5e5abde8d359fd3c7986e8a2a6c8b7f897";
const ACC0_PK_HEX: &str = "c469a5fccfd06f7431928549dad6a651a37063de9a8b5e2ae2322e8c22579c974a6adddfdabf10016958bd1d67b90a5e5abde8d359fd3c7986e8a2a6c8b7f897";
const ACC1_XSK_HEX: &str = "11039b762c860100008098e4f69db9772cf1ce7f979cb01e4dcfc4c7b97a2b7767cd9bc52ab0f9b989cf6e09c10c5995c00194ad94a71b886c3ca96393712b67f93a36ddfd48977ace0d";
const ACC1_XVK_HEX: &str = "21039b762c860100008026ca0d129826c19e74fff1b0e988f1b6f26ad9896dcc10fcc6a1da86627f43088d82ffc4778dbbbfd8c667e4f1850bf85cc19caa380e2d9b8c105149b2fb5709";
const ACC1_SK_HEX: &str = "26ca0d129826c19e74fff1b0e988f1b6f26ad9896dcc10fcc6a1da86627f4308e859aef34ffb9319cb4a463cd1f688a9265e13e0b0fd56994ca623265876740a";
const ACC1_VK_HEX: &str = "26ca0d129826c19e74fff1b0e988f1b6f26ad9896dcc10fcc6a1da86627f43088d82ffc4778dbbbfd8c667e4f1850bf85cc19caa380e2d9b8c105149b2fb5709";
const ACC1_PK_HEX: &str = "abbae63285683be362ee74bf2bf7141a8c0dbb76214daf2db5059b5c77364bc98d82ffc4778dbbbfd8c667e4f1850bf85cc19caa380e2d9b8c105149b2fb5709";
const ACC150_XSK_HEX: &str = "11039b762c8696000080f4e4bd8ebb135dc28a9cea47ce6a36ccd93c805337a7110267331acce793ec34a73c18c145b2eed0a41768347cf19de03a81dbf484253c74b8eeceea9d1b0d01";
const ACC150_XVK_HEX: &str = "21039b762c86960000808a08bf8de8beec8df2b353ba761b0cd9d7e7d480346f48e97ce5ce964549d90b79c8bb1cd312b1fcab974ab86daa0274fbcd78eb13b501f2671fed8f331a3c4c";
const ACC150_SK_HEX: &str = "8a08bf8de8beec8df2b353ba761b0cd9d7e7d480346f48e97ce5ce964549d90bef7ea024b49c6e35b60cf25f0e090d923cc4fed45028a4f2d9c6d8d9217be00b";
const ACC150_VK_HEX: &str = "8a08bf8de8beec8df2b353ba761b0cd9d7e7d480346f48e97ce5ce964549d90b79c8bb1cd312b1fcab974ab86daa0274fbcd78eb13b501f2671fed8f331a3c4c";
const ACC150_PK_HEX: &str = "99bb512e412c73cab91014963eccf59aab70a3466e4f69c183733a64e162b82779c8bb1cd312b1fcab974ab86daa0274fbcd78eb13b501f2671fed8f331a3c4c";
const NONZERO_MASTER_XSK_HEX: &str = "11000000000000000000ee21473f667297506dc80834824d8c1a353bd769949f233344fc289e3a5fa56106160506b94b943619d4ee536301f92f1c3a117ca79f926c7fa51aa4be4f02ba";
const NONZERO_ACC0_XSK_HEX: &str = "110303499b9e000000808357be53090b1773d1832b7b733ed41766669807b4152427ed01a8de60f261a9ee1410fe99c6ee6d945d3d58e3c111dd1450342074a93083e33da5e22056d2f2";
const NONZERO_ACC0_XVK_HEX: &str = "210303499b9e000000803a76061d08b374fcefcb6e4d8a684de7c054f0f8bac717b1029402c5a5f8ab0b0f124947ae07bf25de2a59e12023fb35c20a5e00c683299cbc85bf97e439e86f";
const NONZERO_ACC0_SK_HEX: &str = "3a76061d08b374fcefcb6e4d8a684de7c054f0f8bac717b1029402c5a5f8ab0bdadc1d5d1991028c1e0b0762322e56df0de55ff619981583d4cb74b2c0445e07";
const NONZERO_ACC0_VK_HEX: &str = "3a76061d08b374fcefcb6e4d8a684de7c054f0f8bac717b1029402c5a5f8ab0b0f124947ae07bf25de2a59e12023fb35c20a5e00c683299cbc85bf97e439e86f";
const NONZERO_ACC0_PK_HEX: &str = "8bf6ca35466b8faf4e6e21f16663df777d6fa33a3a46a87eb219768e0c6992200f124947ae07bf25de2a59e12023fb35c20a5e00c683299cbc85bf97e439e86f";

#[test]
fn test_phoenix_hd_deterministic_vectors() {
    let master = phoenix_master_from_seed(&SEED).expect("master key");
    let account_0 = phoenix_account(&master, 0).expect("account 0");
    let account_1 = phoenix_account(&master, 1).expect("account 1");
    let account_150 = phoenix_account(&master, 150).expect("account 150");

    let account_0_xsk = phoenix_account_xsk(&master, 0).expect("account 0 xsk");
    let account_1_xsk = phoenix_account_xsk(&master, 1).expect("account 1 xsk");
    let account_150_xsk =
        phoenix_account_xsk(&master, 150).expect("account 150 xsk");

    assert_eq!(hex::encode(master.as_xsk().to_bytes()), MASTER_XSK_HEX);

    assert_eq!(hex::encode(account_0_xsk.to_bytes()), ACC0_XSK_HEX);
    assert_eq!(
        hex::encode(phoenix_view_key(&account_0_xsk).to_bytes()),
        ACC0_XVK_HEX
    );
    assert_eq!(hex::encode(account_0.sk.to_bytes()), ACC0_SK_HEX);
    assert_eq!(hex::encode(account_0.vk.to_bytes()), ACC0_VK_HEX);
    assert_eq!(hex::encode(account_0.pk.to_bytes()), ACC0_PK_HEX);

    assert_eq!(hex::encode(account_1_xsk.to_bytes()), ACC1_XSK_HEX);
    assert_eq!(
        hex::encode(phoenix_view_key(&account_1_xsk).to_bytes()),
        ACC1_XVK_HEX
    );
    assert_eq!(hex::encode(account_1.sk.to_bytes()), ACC1_SK_HEX);
    assert_eq!(hex::encode(account_1.vk.to_bytes()), ACC1_VK_HEX);
    assert_eq!(hex::encode(account_1.pk.to_bytes()), ACC1_PK_HEX);

    assert_eq!(hex::encode(account_150_xsk.to_bytes()), ACC150_XSK_HEX);
    assert_eq!(
        hex::encode(phoenix_view_key(&account_150_xsk).to_bytes()),
        ACC150_XVK_HEX
    );
    assert_eq!(hex::encode(account_150.sk.to_bytes()), ACC150_SK_HEX);
    assert_eq!(hex::encode(account_150.vk.to_bytes()), ACC150_VK_HEX);
    assert_eq!(hex::encode(account_150.pk.to_bytes()), ACC150_PK_HEX);
}

#[test]
fn test_nonzero_seed_vector() {
    let master = phoenix_master_from_seed(&NONZERO_SEED).expect("master key");
    let account_0 = phoenix_account(&master, 0).expect("account 0");
    let account_0_xsk = phoenix_account_xsk(&master, 0).expect("account 0 xsk");

    assert_eq!(
        hex::encode(master.as_xsk().to_bytes()),
        NONZERO_MASTER_XSK_HEX
    );
    assert_eq!(hex::encode(account_0_xsk.to_bytes()), NONZERO_ACC0_XSK_HEX);
    assert_eq!(
        hex::encode(phoenix_view_key(&account_0_xsk).to_bytes()),
        NONZERO_ACC0_XVK_HEX
    );
    assert_eq!(hex::encode(account_0.sk.to_bytes()), NONZERO_ACC0_SK_HEX);
    assert_eq!(hex::encode(account_0.vk.to_bytes()), NONZERO_ACC0_VK_HEX);
    assert_eq!(hex::encode(account_0.pk.to_bytes()), NONZERO_ACC0_PK_HEX);
}

#[test]
fn test_raw_and_path_round_trips() {
    let master = phoenix_master_from_seed(&SEED).expect("master key");
    let xsk = phoenix_account_xsk(&master, 150).expect("account xsk");
    let xvk = phoenix_view_key(&xsk);

    let xsk_bytes = xsk.to_bytes();
    let xsk_decoded = PhoenixExtendedSpendingKey::from_bytes(&xsk_bytes)
        .expect("xsk should deserialize");
    assert_eq!(xsk_decoded.to_bytes(), xsk_bytes);

    let xvk_bytes = xvk.to_bytes();
    let xvk_decoded = PhoenixExtendedViewingKey::from_bytes(&xvk_bytes)
        .expect("xvk should deserialize");
    assert_eq!(xvk_decoded.to_bytes(), xvk_bytes);

    let path = PhoenixDerivationPath::new(150).expect("valid path");
    let path_str = path.to_string();
    assert_eq!(path_str, "m/32'/744'/150'");
    let decoded_path =
        PhoenixDerivationPath::from_str(&path_str).expect("path should parse");
    assert_eq!(decoded_path, path);
}

#[test]
fn test_invalid_seed_and_paths() {
    assert!(matches!(
        phoenix_master_from_seed(&[0u8; 31]),
        Err(PhoenixHdError::InvalidSeedLength)
    ));
    assert!(phoenix_master_from_seed(&[0u8; 32]).is_ok());
    assert!(matches!(
        phoenix_master_from_seed(&[0u8; MAX_SEED_LEN + 1]),
        Err(PhoenixHdError::InvalidSeedLength)
    ));

    let invalid_paths = [
        "n/32'/744'/0'",
        "m/32/744'/0'",
        "m/32'/744/0'",
        "m/32'/744'/0",
        "m/33'/744'/0'",
        "m/32'/5995'/0'",
        "m/32'//0'",
        "m/32'/744'/2147483648'",
        "m/32'/744'/0'/0'",
    ];

    for path in invalid_paths {
        assert!(
            PhoenixDerivationPath::from_str(path).is_err(),
            "{path} should be invalid"
        );
    }

    let master = phoenix_master_from_seed(&SEED).expect("master key");
    let max_path =
        PhoenixDerivationPath::new(HARDENED_OFFSET - 1).expect("max path");
    assert_eq!(max_path.to_string(), "m/32'/744'/2147483647'");
    assert!(phoenix_account_xsk(&master, HARDENED_OFFSET - 1).is_ok());
    assert!(phoenix_account(&master, HARDENED_OFFSET).is_err());
}

#[test]
fn test_invalid_serialization() {
    let master = phoenix_master_from_seed(&SEED).expect("master key");
    let xsk = phoenix_account_xsk(&master, 0).expect("account xsk");
    let xvk = phoenix_view_key(&xsk);

    let mut bad_xsk = xsk.to_bytes();
    bad_xsk[0] = (PHOENIX_XSK_KIND << 4) | (PHOENIX_HD_VERSION + 1);
    assert!(PhoenixExtendedSpendingKey::from_bytes(&bad_xsk).is_err());
    assert!(PhoenixExtendedSpendingKey::from_bytes(&xvk.to_bytes()).is_err());

    let mut bad_master = master.as_xsk().to_bytes();
    bad_master[2] = 1;
    assert!(PhoenixExtendedSpendingKey::from_bytes(&bad_master).is_err());

    let mut bad_master = master.as_xsk().to_bytes();
    bad_master[6..10].copy_from_slice(&1u32.to_le_bytes());
    assert!(PhoenixExtendedSpendingKey::from_bytes(&bad_master).is_err());

    let mut bad_child = xsk.to_bytes();
    bad_child[6..10].copy_from_slice(&1u32.to_le_bytes());
    assert!(PhoenixExtendedSpendingKey::from_bytes(&bad_child).is_err());

    let mut bad_xvk = xvk.to_bytes();
    bad_xvk[0] = (PHOENIX_XVK_KIND << 4) | (PHOENIX_HD_VERSION + 1);
    assert!(PhoenixExtendedViewingKey::from_bytes(&bad_xvk).is_err());
    assert!(PhoenixExtendedViewingKey::from_bytes(&xsk.to_bytes()).is_err());

    let mut bad_xvk = xvk.to_bytes();
    bad_xvk[42..74].fill(0xff);
    assert!(PhoenixExtendedViewingKey::from_bytes(&bad_xvk).is_err());

    assert!(PhoenixMasterKey::try_from_xsk(master.as_xsk().clone()).is_ok());
    assert!(matches!(
        PhoenixMasterKey::try_from_xsk(xsk),
        Err(PhoenixHdError::InvalidMetadata)
    ));
}

#[test]
fn test_legacy_and_hd_secret_keys_differ() {
    let index = 42u8;
    let master = phoenix_master_from_seed(&SEED).expect("master key");

    let legacy_sk = legacy::derive_phoenix_sk(&SEED, index);
    let hd_sk = phoenix_account(&master, u32::from(index))
        .expect("account")
        .sk;

    assert_ne!(legacy_sk.to_bytes(), hd_sk.to_bytes());
}

#[test]
fn test_existing_phoenix_key_compatibility_and_ownership() {
    let master = phoenix_master_from_seed(&SEED).expect("master key");
    let account = phoenix_account(&master, 1).expect("account");
    let xsk = phoenix_account_xsk(&master, 1).expect("account xsk");
    let xvk = phoenix_view_key(&xsk);

    assert_eq!(PhoenixViewKey::from(&account.sk), account.vk);
    assert_eq!(PhoenixPublicKey::from(&account.sk), account.pk);
    assert_eq!(PhoenixPublicKey::from(&account.vk), account.pk);
    assert_eq!(xvk.view_key(), account.vk);
    assert_eq!(phoenix_public_key_from_xvk(&xvk), account.pk);

    let mut rng = StdRng::seed_from_u64(0x5048_5848_4431);
    let r = JubJubScalar::random(&mut rng);
    let stealth = account.pk.gen_stealth_address(&r);

    assert!(account.sk.owns(&stealth));
    assert!(account.vk.owns(&stealth));
}

#[test]
fn test_migration_helpers_preserve_profile_mapping() {
    let profile = 5u8;
    let mapping =
        phoenix_migration_account(&SEED, profile).expect("migration account");

    assert_eq!(mapping.profile, profile);
    assert_eq!(mapping.hd.path.to_string(), "m/32'/744'/5'");
    assert_ne!(
        mapping.legacy.sk.to_bytes(),
        mapping.hd.sk.to_bytes(),
        "legacy and HD keys must differ for migration"
    );

    let mappings =
        phoenix_dual_scan_accounts(&SEED, 3..6).expect("dual scan accounts");
    assert_eq!(mappings.len(), 3);
    assert_eq!(mappings[0].profile, 3);
    assert_eq!(mappings[1].profile, 4);
    assert_eq!(mappings[2].profile, 5);
}
