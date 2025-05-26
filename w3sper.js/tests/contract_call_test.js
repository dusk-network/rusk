// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {seeder, test, Treasury} from "./harness.js";
import {AccountSyncer, Bookkeeper, Network, ProfileGenerator} from "@dusk/w3sper";

test("account contract call transfer", async () => {
    const network = await Network.connect("http://localhost:8080/");
    // const network = await Network.connect("https://testnet.nodes.dusk.network/");
    const profiles = new ProfileGenerator(seeder);
    const users = await Promise.all([profiles.default, profiles.next()]);
    const accounts = new AccountSyncer(network);
    const treasury = new Treasury(users);

    await treasury.update({accounts});

    const bookkeeper = new Bookkeeper(treasury);

    let transfer = bookkeeper
        .as(users[1])
        .transfer(1n)
        .to(users[0].account)
        .memo(null)
        .fn_name("get_version")
        .fn_args([])
        .contract_id([0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        .gas({limit: 500_000_000n});

    let {hash} = await network.execute(transfer);

    let evt = await network.transactions.withId(hash).once.executed();

    console.log("evt:", evt);
    console.log("evt.payload:", evt.payload);

    await treasury.update({accounts});

    await network.disconnect();
});
