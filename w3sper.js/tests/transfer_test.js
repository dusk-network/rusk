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

test.withLocalWasm = "release";

test("balances", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = await Promise.all([profiles.default, profiles.next()]);

  let url = new URL("on/transactions/executed", network.url);

  let response = await fetch(url, {
    headers: {
      "rusk-version": "0.8.0",
      "rusk-session-id": network.sessionId,
    },
  }).catch(console.error);
  console.log(response);

  const addresses = new AddressSyncer(network);

  const treasury = new Treasury(users);
  const from = Bookmark.from(0n);

  await treasury.update({ addresses, from });

  const bookkeeper = new Bookkeeper(treasury);

  let addressBalances = await Promise.all(
    users.map((user) => bookkeeper.balance(user.address)),
  );

  console.log(addressBalances[0].value, addressBalances[1].value);

  const transfer = bookkeeper
    .transfer(1n)
    .obfuscated()
    .from(users[1].address)
    .to(users[0].address)
    .gas(new Gas({ limit: 500_000_000n }));

  const tx = await network.execute(transfer);

  console.log("hash:", tx.hash);
  console.log("nullifier", tx.nullifiers);

  await new Promise((r) => setTimeout(r, 30_000));
  await network.disconnect();
});
