// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used by Dusk's license contract.

use crate::{reserved, ContractId};

/// ID of the genesis license contract
pub const LICENSE_CONTRACT: ContractId = reserved(0x3);
