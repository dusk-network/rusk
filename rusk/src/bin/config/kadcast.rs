// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use kadcast::config::Config;
use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct KadcastConfig(Config);

impl From<KadcastConfig> for Config {
    fn from(conf: KadcastConfig) -> Self {
        conf.0
    }
}

impl KadcastConfig {
    pub(crate) fn merge(&mut self, arg: &Args) {
        if let Some(public_address) = &arg.kadcast_public_address {
            self.0.public_address = public_address.into();
        };
        if let Some(listen_address) = &arg.kadcast_listen_address {
            self.0.listen_address = Some(listen_address.into());
        };
        if let Some(bootstrapping_nodes) = arg.kadcast_bootstrap.clone() {
            self.0.bootstrapping_nodes = bootstrapping_nodes
        };
        if let Some(network_id) = arg.kadcast_network_id {
            self.0.kadcast_id = Some(network_id)
        };
    }
}
