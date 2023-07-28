// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::ledger::SpentTransaction as Tx;

use super::*;

pub async fn tx_by_hash(ctx: &Ctx, hash: String) -> OptResult<Tx> {
    let hash = hex::decode(hash)?;
    let tx = ctx.read().await.view(|t| t.get_ledger_tx_by_hash(&hash))?;
    Ok(tx)
}
