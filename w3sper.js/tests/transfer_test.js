// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  Network,
  Gas,
  ProfileGenerator,
  Bookkeeper,
  Bookmark,
  AddressSyncer,
  AccountSyncer,
} from "../src/mod.js";

import { test, assert, seeder, Treasury } from "./harness.js";
import * as ProtocolDriver from "../src/protocol-driver/mod.js";

test.withLocalWasm = "debug";

test("balances", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = await Promise.all([profiles.next(), profiles.next()]);

  const addresses = new AddressSyncer(network);

  const treasury = new Treasury(users);
  const from = Bookmark.from(1400n);

  await treasury.read({ addresses, from });

  const bookkeeper = new Bookkeeper(treasury);

  let addressBalances = await Promise.all(
    users.map((user) => bookkeeper.balance(user.address)),
  );
  treasury.cached = addressBalances;

  assert.equal(addressBalances[0].value, 1323002002157n);
  assert.equal(addressBalances[1].value, 512720219906168n);

  const transfer = bookkeeper
    .transfer(1000n)
    .obfuscated()
    .from(users[1].address)
    .to(users[0].address);

  const gas = new Gas();

  assert.equal(gas.limit, Gas.DEFAULT_LIMIT);
  assert.equal(gas.price, Gas.DEFAULT_PRICE);

  const result = await network.execute(transfer, gas);

  await network.disconnect();
});
