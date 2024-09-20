// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  test,
  assert,
} from "http://rawcdn.githack.com/mio-mini/test-harness/0.1.0/mod.js";

import {
  Network,
  ProfileGenerator,
  Bookkeeper,
  Bookmark,
  StateSyncer,
} from "../src/mod.js";

const once = (target, topic) =>
  new Promise((resolve) =>
    target.addEventListener(topic, resolve, { once: true }),
  );

// Define a seed for deterministic profile generation
const SEED = new Uint8Array([
  153, 16, 102, 99, 133, 196, 55, 237, 42, 2, 163, 116, 233, 89, 10, 115, 19,
  81, 140, 31, 38, 81, 10, 46, 118, 112, 151, 244, 145, 90, 145, 168, 214, 242,
  68, 123, 116, 76, 223, 56, 200, 60, 188, 217, 34, 113, 55, 172, 27, 255, 184,
  55, 143, 233, 109, 20, 137, 34, 20, 196, 252, 117, 221, 221,
]);

const seeder = () => SEED;

// Test case for default profile
test("Transfer", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const profiles = new ProfileGenerator(seeder);

  const bookmark = Bookmark.from(1300);
  const syncer = new StateSyncer(network);

  const users = await Promise.all([profiles.default, profiles.next()]);
  // const entries = await syncer.notes(users, { from: bookmark });

  await syncer.accounts(users);

  // let allEntries = new Map();
  // for await (let [owned, syncInfo] of entries) {
  //   allEntries = new Map([...allEntries, ...owned]);
  // }

  // const bookkeeper = new Bookkeeper({
  //   address(_profile) {
  //     return allEntries;
  //   },
  //   account() {},
  // });

  // const balances = await Promise.all(
  //   users.map((user) => bookkeeper.balance(user.address)),
  // );
  // assert.equal(balances[0].value, 0n);
  // assert.equal(balances[1].value, 4251806104962n);

  // bookkeeper.transfer({
  //   from: users[0].address,
  //   to: users[1].address,
  //   amount: 1000n,
  // });

  // bookkeeper.transfer(1000n, {
  //   from: users[0].address,
  //   to: users[1].address,
  // });

  await network.disconnect();
});