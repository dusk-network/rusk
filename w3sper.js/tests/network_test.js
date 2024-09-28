// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  Network,
  ProfileGenerator,
  Bookkeeper,
  AddressSyncer,
  AccountSyncer,
} from "../src/mod.js";

import { test, assert } from "./harness.js";

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

test.withLocalWasm = "release";

// Test case for default profile
test("Network connection", async () => {
  const network = new Network("http://localhost:8080/");

  assert.ok(!network.connected);
  assert.equal(network.nodeInfo, null);

  await network.connect();
  assert.ok(network.connected);
  assert.equal(Object.keys(network.nodeInfo), [
    "bootstrappingNodes",
    "chainId",
    "kadcastAddress",
    "version",
    "versionBuild",
  ]);

  await network.disconnect();
  assert.ok(!network.connected);
  assert.equal(network.nodeInfo, null);
});

test("Network block height", async () => {
  const network = await Network.connect("http://localhost:8080/");

  assert.ok((await network.blockHeight) > 0);

  await network.disconnect();
});

test("Network synchronization", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const owners = await Promise.all([
    profiles.default,
    profiles.next(),
    profiles.next(),
  ]);

  const addresses = new AddressSyncer(network);
  const accounts = new AccountSyncer(network);

  let iterationOwnedCountTotal = 0;
  addresses.addEventListener("synciteration", ({ detail }) => {
    const { ownedCount } = detail;

    iterationOwnedCountTotal += ownedCount;
  });

  let ownedNotes = new Map();
  for await (let [notes, _syncInfo] of await addresses.notes(owners, {
    from: 0n,
  })) {
    ownedNotes = new Map([...ownedNotes, ...notes]);
  }

  assert.equal(iterationOwnedCountTotal, 1857);
  assert.equal(ownedNotes.size, iterationOwnedCountTotal);

  const balances = await accounts.balances(owners);

  const bookkeeper = new Bookkeeper({
    address(_profile) {
      return ownedNotes;
    },
    account(profile) {
      return balances.at(+profile);
    },
  });

  const addressBalances = await Promise.all(
    owners.map((owner) => bookkeeper.balance(owner.address)),
  );

  assert.equal(addressBalances[0].value, 1026179647718621n);
  assert.equal(addressBalances[1].value, 1419179830115057n);
  assert.equal(addressBalances[2].value, 512720219906168n);

  const accountBalances = await Promise.all(
    owners.map((owner) => bookkeeper.balance(owner.account)),
  );

  assert.equal(accountBalances[0].value, 10100000000n);
  assert.equal(accountBalances[1].value, 8800000000n);
  assert.equal(accountBalances[2].value, 6060000000n);

  await network.disconnect();
});
