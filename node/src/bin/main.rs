// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::chain::ChainSrv;
use node::database::{rocksdb, DB};
use node::mempool::MempoolSrv;
use node::network::Kadcast;
use node::{LongLivedService, Node};

#[tokio::main]
pub async fn main() {
    node::enable_log(tracing::Level::INFO);

    // Set up a node where:
    // transport layer is Kadcast with message ids from 0 to 255
    // persistence layer is rocksdb
    type Services = dyn LongLivedService<Kadcast<255>, rocksdb::Backend>;

    // Select list of services to enable
    let service_list: Vec<Box<Services>> =
        vec![Box::<MempoolSrv>::default(), Box::<ChainSrv>::default()];

    let net = Kadcast::new(kadcast::config::Config::default());
    let db = rocksdb::Backend::create_or_open("".to_string());

    // node spawn_all is the entry point
    if let Err(e) = Node::new(net, db).spawn_all(service_list).await {
        tracing::error!("node terminated with err: {}", e);
    }
}
