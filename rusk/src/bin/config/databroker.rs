// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::{Deserialize, Serialize};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct DataBrokerConfig(node::databroker::conf::Params);

impl From<DataBrokerConfig> for node::databroker::conf::Params {
    fn from(conf: DataBrokerConfig) -> Self {
        conf.0
    }
}

impl DataBrokerConfig {
    pub fn merge(&mut self, args: &Args) {
        if let Some(delay_on_resp_msg) = args.delay_on_resp_msg {
            self.0.delay_on_resp_msg = Some(delay_on_resp_msg);
        };
    }
}
