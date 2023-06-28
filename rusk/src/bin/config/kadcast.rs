// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{Arg, ArgMatches, Command};
use kadcast::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct KadcastConfig(Config);

impl From<KadcastConfig> for Config {
    fn from(conf: KadcastConfig) -> Self {
        conf.0
    }
}

impl KadcastConfig {
    pub(crate) fn merge(&mut self, matches: &ArgMatches) {
        if let Some(public_address) = matches.value_of("kadcast_public_address")
        {
            self.0.public_address = public_address.into();
        };
        if let Some(listen_address) = matches.value_of("kadcast_listen_address")
        {
            self.0.listen_address = Some(listen_address.into());
        };
        if let Some(bootstrapping_nodes) =
            matches.values_of("kadcast_bootstrap")
        {
            self.0.bootstrapping_nodes =
                bootstrapping_nodes.map(|s| s.into()).collect();
        };
        self.0.auto_propagate = matches.is_present("kadcast_autobroadcast");
    }

    pub fn inject_args(command: Command<'_>) -> Command<'_> {
        command.arg(
            Arg::new("kadcast_public_address")
                .long("kadcast_public_address")
                .long_help("This is the address where other peer can contact you. 
    This address MUST be accessible from any peer of the network")
                .help("Public address you want to be identified with. Eg: 193.xxx.xxx.198:9999")
                .env("KADCAST_PUBLIC_ADDRESS")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::new("kadcast_listen_address")
                .long("kadcast_listen_address")
                .long_help("This address is the one bound for the incoming connections. 
    Use this argument if your host is not publicly reachable from other peer in
    the network (Eg: if you are behind a NAT)
    If this is not specified, the public address is used for binding incoming connection")
                .help("Optional internal address to listen incoming connections. Eg: 127.0.0.1:9999")
                .env("KADCAST_LISTEN_ADDRESS")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::new("kadcast_bootstrap")
                .long("kadcast_bootstrap")
                .env("KADCAST_BOOTSTRAP")
                .multiple_occurrences(true)
                .help("Kadcast list of bootstrapping server addresses")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::new("kadcast_autobroadcast")
                .long("kadcast_autobroadcast")
                .env("KADCAST_AUTOBROADCAST")
                .help("If used then the received messages are automatically re-broadcasted")
                .takes_value(false)
                .required(false),
        )
    }
}
