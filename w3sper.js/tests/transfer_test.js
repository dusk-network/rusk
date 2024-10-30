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

test.withLocalWasm = "release";

test("accounts", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = await Promise.all([profiles.default, profiles.next()]);

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookkeeper = new Bookkeeper(treasury);

  const balances = [
    await bookkeeper.balance(users[0].account),
    await bookkeeper.balance(users[1].account),
  ];

  const transfer = bookkeeper
    .transfer(77n)
    .from(users[1].account)
    .to(users[0].account)
    .gas(new Gas({ limit: 500_000_000n }));

  const { hash } = await network.execute(transfer);

  await network.transactions.withId(hash).once.executed();

  await treasury.update({ accounts });

  let newBalances = [
    await bookkeeper.balance(users[0].account),
    await bookkeeper.balance(users[1].account),
  ];

  assert.equal(newBalances[0].value, balances[0].value + 77n);
  assert.equal(newBalances[1].nonce, balances[1].nonce + 1n);

  await network.disconnect();
});

test("addresses", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = await Promise.all([profiles.default, profiles.next()]);

  const addresses = new AddressSyncer(network);

  const treasury = new Treasury(users);
  let from = Bookmark.from(0n);

  await treasury.update({ addresses, from });

  const bookkeeper = new Bookkeeper(treasury);

  const balances = [
    await bookkeeper.balance(users[0].address),
    await bookkeeper.balance(users[1].address),
  ];

  const transfer = bookkeeper
    .transfer(11n)
    .obfuscated()
    .from(users[1].address)
    .to(users[0].address)
    .gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  await network.transactions.withId(hash).once.executed();

  await treasury.update({ addresses });

  let newBalances = [
    await bookkeeper.balance(users[0].address),
    await bookkeeper.balance(users[1].address),
  ];

  assert.equal(newBalances[0].value, balances[0].value + 11n);

  await network.disconnect();
});
