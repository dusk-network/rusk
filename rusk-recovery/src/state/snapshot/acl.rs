// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_bytes::Serializable;
use serde_derive::{Deserialize, Serialize};

use super::Wrapper;

#[derive(Serialize, Deserialize, Default, PartialEq, Eq)]
pub(super) struct Acl {
    pub(super) stake: Users,
}

#[derive(Serialize, Deserialize, Default, PartialEq, Eq)]
pub(super) struct Users {
    pub(super) owners: Vec<Wrapper<BlsPublicKey, { BlsPublicKey::SIZE }>>,
    pub(super) allowlist: Vec<Wrapper<BlsPublicKey, { BlsPublicKey::SIZE }>>,
}
