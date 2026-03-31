// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(
    not(any(feature = "chain", feature = "prover", test)),
    allow(dead_code, unused_imports)
)]

pub(crate) mod event;
mod request;
mod response;
mod subscription;
pub(crate) mod ws;

pub(crate) use request::{ParsedRuesRequest, validate_rusk_version_headers};
pub(crate) use subscription::SubscriptionAction;
#[cfg(any(feature = "chain", test))]
pub(crate) use subscription::{subscribe, unsubscribe};
