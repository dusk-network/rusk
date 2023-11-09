// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(any(feature = "recovery-state", feature = "recovery-keys"))]
mod command;
#[cfg(feature = "recovery-state")]
mod state;

use std::path::PathBuf;

use clap::builder::PossibleValuesParser;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    author="Dusk Network B.V. All Rights Reserved.",
    version = &rusk::VERSION_BUILD[..],
    about = "Rusk server node",
)]
pub struct Args {
    /// Sets the profile path
    #[clap(long, short, env = "RUSK_CONFIG_TOML", value_parser)]
    pub config: Option<PathBuf>,

    /// Output log level
    #[clap(long)]
    pub log_level: Option<tracing::Level>,

    // Change the log format accordingly
    #[clap(long, value_parser = PossibleValuesParser::new(["coloured", "plain", "json"]))]
    pub log_type: Option<String>,

    /// Add log filter(s)
    #[clap(long)]
    pub log_filter: Option<String>,

    /// Sets the profile path
    #[clap(long, value_parser)]
    pub profile: Option<PathBuf>,

    #[cfg(feature = "ephemeral")]
    /// Ephemeral state file (archive)
    #[clap(short, long = "state", value_parser)]
    pub state_path: Option<PathBuf>,

    #[clap(long, value_parser)]
    /// path to blockchain database
    pub db_path: Option<PathBuf>,

    #[clap(long, value_parser)]
    /// path to encrypted BLS keys
    pub consensus_keys_path: Option<PathBuf>,

    #[clap(long)]
    /// Delay in milliseconds to mitigate UDP drops for DataBroker service in
    /// localnet
    pub delay_on_resp_msg: Option<u64>,

    #[clap(long)]
    /// Address http server should listen on
    pub http_listen_addr: Option<String>,

    #[clap(long, env = "KADCAST_BOOTSTRAP", verbatim_doc_comment)]
    /// Kadcast list of bootstrapping server addresses
    pub kadcast_bootstrap: Option<Vec<String>>,

    #[clap(long, env = "KADCAST_PUBLIC_ADDRESS", verbatim_doc_comment)]
    /// Public address you want to be identified with. Eg: 193.xxx.xxx.198:9999
    ///
    /// This is the address where other peer can contact you.
    /// This address MUST be accessible from any peer of the network"
    pub kadcast_public_address: Option<String>,

    #[clap(long, env = "KADCAST_LISTEN_ADDRESS", verbatim_doc_comment)]
    /// Optional internal address to listen incoming connections. Eg:
    /// 127.0.0.1:9999
    ///
    /// This address is the one bound for the incoming connections.
    /// Use this argument if your host is not publicly reachable from other
    /// peer in the network (Eg: if you are behind a NAT)
    /// If this is not specified, the public address is used for binding
    /// incoming connection
    pub kadcast_listen_address: Option<String>,

    #[clap(short = 'n', long = "network-id")]
    /// Kadcast network id
    pub kadcast_network_id: Option<u8>,

    #[cfg(any(feature = "recovery-state", feature = "recovery-keys"))]
    /// Command
    #[clap(subcommand)]
    pub command: Option<command::Command>,
}
