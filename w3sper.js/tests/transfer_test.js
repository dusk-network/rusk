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

  // const TRANSFER =
  //   "0100000000000000000000000000000000000000000000000000000000000000";

  // const url = new URL(`/on/contracts:${TRANSFER}/root`, network.url);

  // const req = new Request(url, {
  //   headers: { "Content-Type": "application/octet-stream" },
  //   method: "POST",
  // });

  // const response = await network.dispatch(req);
  // const buffer = await response.arrayBuffer();
  // const root = new Uint8Array(await buffer);
  // console.log(root);

  // await ProtocolDriver.root(root);

  // return;

  const users = await Promise.all([profiles.default, profiles.next()]);

  const addresses = new AddressSyncer(network);

  const treasury = new Treasury(users);
  const from = Bookmark.from(0n);

  await treasury.read({ addresses, from });

  const bookkeeper = new Bookkeeper(treasury);

  let addressBalances = await Promise.all(
    users.map((user) => bookkeeper.balance(user.address)),
  );
  treasury.cached = addressBalances;

  console.log(addressBalances[0].value, addressBalances[1].value);
  // assert.equal(addressBalances[0].value, 1323002002157n);
  // assert.equal(addressBalances[1].value, 512720219906168n);

  const transfer = bookkeeper
    .transfer(1n)
    .obfuscated()
    .from(users[1].address)
    .to(users[0].address)
    .gas(new Gas({ limit: 500_000_000n }));

  const result = await network.execute(transfer);

  await network.disconnect();
});
