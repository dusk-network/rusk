// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node::database::rocksdb::Backend;
use node::network::Kadcast;

use std::sync::Arc;

use crate::Rusk;

#[derive(Clone)]
pub struct RuskNode(pub node::Node<Kadcast<255>, Backend, Rusk>);
impl RuskNode {
    pub fn db(&self) -> Arc<tokio::sync::RwLock<Backend>> {
        self.0.database() as Arc<tokio::sync::RwLock<Backend>>
    }
}
