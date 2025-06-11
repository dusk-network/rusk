// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {assert, seeder, test, Treasury} from "./harness.js";
import {AccountSyncer, Bookkeeper, Network, ProfileGenerator} from "@dusk/w3sper";

const GAS_LIMIT = 500_000_000n;
const METHOD = "get_version";

test("account contract call transfer", async () => {
    const network = await Network.connect("http://localhost:8080/");
    const profiles = new ProfileGenerator(seeder);
    const users = await Promise.all([profiles.default, profiles.next()]);
    const accounts = new AccountSyncer(network);
    const treasury = new Treasury(users);

    await treasury.update({accounts});

    const bookkeeper = new Bookkeeper(treasury);

    const payload = {
        fnName: METHOD,
        fnArgs: [],
        contractId: [0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    };

    let transfer = bookkeeper
        .as(users[1])
        .transfer(1n)
        .to(users[0].account)
        .payload(payload)
        .gas({limit: GAS_LIMIT});

    let {hash} = await network.execute(transfer);

    let evt = await network.transactions.withId(hash).once.executed();

    assert.ok(!evt.payload.err, "contract call error");
    assert.ok(evt.payload.gas_spent < GAS_LIMIT, "gas limit reached");
    const { contract, fn_name } = evt.call();
    assert.equal(contract, "0200000000000000000000000000000000000000000000000000000000000000");
    assert.equal(fn_name, METHOD);

    await treasury.update({accounts});

    await network.disconnect();
});
