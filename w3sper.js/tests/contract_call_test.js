// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {assert, seeder, test, Treasury} from "./harness.js";
import {AccountSyncer, Bookkeeper, Network, ProfileGenerator} from "@dusk/w3sper";

test("withdraw 0 as stake reward", async () => {
    const network = await Network.connect("http://localhost:8080/");
    const profiles = new ProfileGenerator(seeder);

    const users = [await profiles.default, await profiles.next()];

    const accounts = new AccountSyncer(network);

    const treasury = new Treasury(users);
    const bookkeeper = new Bookkeeper(treasury);
    const bookentry = bookkeeper.as(users[0]);

    await treasury.update({ accounts });

    const transfer = bookentry.withdraw(0n).gas({ limit: 500_000_000n });

    await assert.reject(
        async () => await network.execute(transfer),
        RangeError,
        "Can't withdraw an empty reward amount.",
    );

    await network.disconnect();
});
