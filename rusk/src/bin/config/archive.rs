// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct ArchiveConfig(pub(crate) node::archive::conf::Params);

impl From<ArchiveConfig> for node::archive::conf::Params {
    fn from(conf: ArchiveConfig) -> Self {
        conf.0
    }
}
