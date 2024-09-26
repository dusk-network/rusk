// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::Subcommand;

#[derive(PartialEq, Eq, Hash, Clone, Subcommand, Debug)]
pub enum ChainCommand {
    /// Revert chain state to last final state
    Revert,
}
