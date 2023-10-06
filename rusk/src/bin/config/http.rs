// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct HttpConfig {
    #[serde(default = "bool::default")]
    pub listen: bool,
    listen_address: Option<String>,
}

impl HttpConfig {
    pub fn listen_addr(&self) -> String {
        self.listen_address
            .clone()
            .unwrap_or("127.0.0.1:8080".into())
    }

    pub(crate) fn merge(&mut self, matches: &ArgMatches) {
        // Overwrite config ws-listen-addr
        if let Some(ws_listen_addr) =
            matches.get_one::<String>("ws-listen-addr")
        {
            self.listen_address = Some(ws_listen_addr.into());
        }
    }

    pub fn inject_args(command: Command) -> Command {
        command.arg(
            Arg::new("ws-listen-addr")
                .long("ws-listen-addr")
                .value_name("WS_LISTEN_ADDR")
                .help("Address websocket should listen on")
                .num_args(1),
        )
    }
}
