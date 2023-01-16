// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::{
    chain::ChainSrv,
    mempool::MempoolSrv,
    network::{self},
    LongLivedService, Node,
};

#[tokio::main]
pub async fn main() {
    node::enable_log(tracing::Level::INFO);

    type Services = dyn LongLivedService<network::Kadcast<255>>;

    // Select list of services to enable
    let service_list: Vec<Box<Services>> =
        vec![Box::<MempoolSrv>::default(), Box::<ChainSrv>::default()];

    let net = network::Kadcast::new(kadcast::config::Config::default());

    // node spawn_all is the entry point
    if let Err(e) = Node::new(net).spawn_all(service_list).await {
        tracing::error!("node terminated with err: {}", e);
    }
}
