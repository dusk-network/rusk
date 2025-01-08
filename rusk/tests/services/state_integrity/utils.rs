// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::services::state_integrity::base_info::BaseInfo;
use crate::services::state_integrity::page_tree::ContractMemTree;
use dusk_core::abi::ContractId;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub const MAIN_DIR: &str = "main";
pub const EDGE_DIR: &str = "edge";
pub const ELEMENT_FILE: &str = "element";
pub const LEAF_DIR: &str = "leaf";
pub const BASE_FILE: &str = "base";

pub fn contract_id_from_hex<S: AsRef<str>>(contract_id: S) -> ContractId {
    let bytes: [u8; 32] = hex::decode(contract_id.as_ref())
        .expect("Hex decoding of contract id string should succeed")
        .try_into()
        .expect("Contract id string conversion should succeed");
    ContractId::from_bytes(bytes)
}

fn base_from_path<P: AsRef<Path>>(path: P) -> io::Result<BaseInfo> {
    let path = path.as_ref();

    let base_info_bytes = fs::read(path)?;
    let base_info = rkyv::from_bytes(&base_info_bytes).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid base info file \"{path:?}\": {err}"),
        )
    })?;

    Ok(base_info)
}

pub fn position_from_contract(contract: &ContractId) -> u64 {
    let pos = contract
        .as_bytes()
        .chunks(4)
        .map(|chunk| {
            let mut bytes = [0; 4];
            bytes.copy_from_slice(chunk);
            u32::from_le_bytes(bytes)
        })
        .fold(0, u32::wrapping_add);

    pos as u64
}

/// Returns path to a file at a given level, and, if not present
/// tries lower levels form the list until found or level zero reached
/// note: no commit id here, edge is oblivious to commits,
/// this function implements a moving main
/// note: may return path of a file which does not exist, in such
/// case it will be a path at level zero
pub fn find_file_path_at_level(
    main_dir: impl AsRef<Path>,
    level: u64,
    contract_id_str: impl AsRef<str>,
    filename: impl AsRef<str>,
    levels: &[u64], // sorted ascending
) -> PathBuf {
    let postfix =
        PathBuf::from(contract_id_str.as_ref()).join(filename.as_ref());
    assert!(!levels.is_empty(), "level list must not be empty");
    assert_eq!(levels[0], 0u64, "level 0 must be first in levels");
    let mut file_path = PathBuf::new();
    for l in levels.iter().rev() {
        if *l > level {
            continue;
        }
        file_path = if *l != 0 {
            main_dir
                .as_ref()
                .join(EDGE_DIR)
                .join(format!("{}", *l))
                .join(&postfix)
        } else {
            main_dir.as_ref().join(&postfix)
        };
        if file_path.is_file() {
            break;
        }
    }
    file_path
}

/// Returns path to a file representing a given commit and element.
///
/// Requires a contract's leaf path and a main state path.
/// Progresses recursively via bases of commits.
pub fn find_element(
    commit: Option<[u8; 32]>,
    leaf_path: impl AsRef<Path>,
    main_path: impl AsRef<Path>,
) -> Option<PathBuf> {
    if let Some(hash) = commit {
        let hash_hex = hex::encode(hash);
        let path = leaf_path.as_ref().join(&hash_hex).join(ELEMENT_FILE);
        if path.is_file() {
            Some(path)
        } else {
            let base_info_path =
                main_path.as_ref().join(hash_hex).join(BASE_FILE);
            let index = base_from_path(base_info_path).ok()?;
            find_element(index.maybe_base, leaf_path, main_path)
        }
    } else {
        None
    }
}

pub fn calculate_root<'a>(
    entries: impl Iterator<Item = (&'a [u8; 32], &'a u64)>,
) -> [u8; 32] {
    let mut tree = ContractMemTree::new();
    for (hash, int_pos) in entries {
        tree.insert(*int_pos, *hash);
    }
    let r = (*tree.root()).as_bytes().clone();
    r
}

pub fn calculate_root_pos_32<'a>(
    entries: impl Iterator<Item = (&'a [u8; 32], &'a u32)>,
) -> [u8; 32] {
    let mut tree = ContractMemTree::new();
    for (hash, int_pos) in entries {
        let int_pos = *int_pos as u64;
        tree.insert(int_pos, *hash);
    }
    let r = (*tree.root()).as_bytes().clone();
    r
}
