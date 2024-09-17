// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "node")]
mod node;
#[cfg(feature = "node")]
pub use node::RuskNodeBuilder as Builder;

#[cfg(not(feature = "node"))]
mod http_only;
#[cfg(not(feature = "node"))]
pub use http_only::RuskHttpBuilder as Builder;
