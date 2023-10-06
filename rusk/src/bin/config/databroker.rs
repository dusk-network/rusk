// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct DataBrokerConfig(node::databroker::conf::Params);

impl From<DataBrokerConfig> for node::databroker::conf::Params {
    fn from(conf: DataBrokerConfig) -> Self {
        conf.0
    }
}

impl DataBrokerConfig {
    pub fn merge(&mut self, matches: &ArgMatches) {
        if let Some(delay_on_resp_msg) =
            matches.get_one::<String>("delay_on_resp_msg")
        {
            match delay_on_resp_msg.parse() {
                Ok(delay_on_resp_msg) => {
                    self.0.delay_on_resp_msg = Some(delay_on_resp_msg);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to parse delay_on_resp_msg: {:?}",
                        e
                    );
                }
            }
        };
    }

    pub fn inject_args(command: Command) -> Command {
        command.arg(
                Arg::new("delay_on_resp_msg")
                    .long("delay_on_resp_msg")
                    .help("Delay in milliseconds to mitigate UDP drops for DataBroker service in localnet")
                    .env("DELAY_ON_RESP_MSG")
                    .num_args(1)
            )
    }
}
