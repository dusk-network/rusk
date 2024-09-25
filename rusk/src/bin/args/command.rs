// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(any(feature = "recovery-state", feature = "recovery-keys"))]
pub mod recovery;

#[cfg(feature = "chain")]
pub mod chain;

use clap::Subcommand;

#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Eq, Hash, Clone, Subcommand, Debug)]
pub enum Command {
    #[cfg(any(feature = "recovery-state", feature = "recovery-keys"))]
    #[clap(subcommand)]
    Recovery(recovery::RecoveryCommand),

    #[cfg(feature = "chain")]
    #[clap(subcommand)]
    Chain(chain::ChainCommand),
}
