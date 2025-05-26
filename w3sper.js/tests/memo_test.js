// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {assert, getLocalWasmBuffer, seeder, test, Treasury} from "./harness.js";
import {AccountSyncer, AddressSyncer, Bookkeeper, Network, ProfileGenerator, useAsProtocolDriver} from "@dusk/w3sper";

test("address memo transfer", async () => {
    const { cleanup } = useAsProtocolDriver(await getLocalWasmBuffer()); // Temporarily needed, while the node doesn't serve the latest WASM.
    // const network = await Network.connect("http://localhost:8080/");
    const network = await Network.connect("https://testnet.nodes.dusk.network/");
    const profiles = new ProfileGenerator(seeder);
    const users = await Promise.all([profiles.default, profiles.next()]);
    const addresses = new AddressSyncer(network);
    const treasury = new Treasury(users);

    await treasury.update({ addresses });

    const bookkeeper = new Bookkeeper(treasury);

    let transfer = bookkeeper
        .as(users[1])
        .transfer(1n)
        .to(users[0].address)
        .memo(new Uint8Array([2, 4, 8, 16]))
        .gas({ limit: 500_000_000n });

    let { hash } = await network.execute(transfer);

    let evt = await network.transactions.withId(hash).once.executed();

    assert.equal([...evt.memo()], [2, 4, 8, 16]);

    await treasury.update({ addresses });

    transfer = bookkeeper
        .as(users[1])
        .transfer(1n)
        .to(users[0].address)
        .memo("Tarapia Tapioco, come fosse stringa")
        .gas({ limit: 500_000_000n });

    ({ hash } = await network.execute(transfer));

    evt = await network.transactions.withId(hash).once.executed();

    // deno-fmt-ignore
    assert.equal(
        [...evt.memo()],
        [
            84, 97, 114, 97, 112, 105, 97, 32, 84, 97, 112, 105, 111, 99, 111, 44, 32,
            99, 111, 109, 101, 32, 102, 111, 115, 115, 101, 32, 115, 116, 114, 105,
            110, 103, 97,
        ]
    );

    assert.equal(
        evt.memo({ as: "string" }),
        "Tarapia Tapioco, come fosse stringa"
    );

    await network.disconnect();
    await cleanup(); // Remove when useAsProtocolDriver is removed.
});
