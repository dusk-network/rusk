// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use tokio::runtime::Handle;
use tokio::task::block_in_place;

pub(crate) trait Block {
    fn wait(self) -> <Self as futures::Future>::Output
    where
        Self: Sized,
        Self: futures::Future,
    {
        block_in_place(move || Handle::current().block_on(self))
    }
}

impl<F, T> Block for F where F: futures::Future<Output = T> {}
