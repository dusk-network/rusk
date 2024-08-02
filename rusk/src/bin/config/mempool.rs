// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct MempoolConfig(node::mempool::conf::Params);

impl From<MempoolConfig> for node::mempool::conf::Params {
    fn from(conf: MempoolConfig) -> Self {
        conf.0
    }
}
