// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "chain")]
mod node;
#[cfg(feature = "chain")]
pub use node::RuskNodeBuilder as Builder;

#[cfg(not(feature = "chain"))]
mod http_only;
#[cfg(not(feature = "chain"))]
pub use http_only::RuskHttpBuilder as Builder;
