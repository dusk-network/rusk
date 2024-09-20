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
  const syncer = new StateSyncer(network);

  let iterationOwnedCountTotal = 0;
  syncer.addEventListener("synciteration", ({ detail }) => {
    const { ownedCount } = detail;

    iterationOwnedCountTotal += ownedCount;
  });

  const profiles = new ProfileGenerator(seeder);

  const owners = await Promise.all([
    profiles.default,
    profiles.next(),
    profiles.next(),
  ]);

  const notes = await syncer.notes(owners, { from: 0n });
  //const accounts = await syncer.accounts(owners);

  let ownedNotes = new Map();
  for await (let [owned, _syncInfo] of notes) {
    ownedNotes = new Map([...ownedNotes, ...owned]);
  }

  assert.ok(iterationOwnedCountTotal, 1857);
  assert.ok(ownedNotes.size, iterationOwnedCountTotal);

  const bookkeeper = new Bookkeeper({
    address(_profile) {
      return ownedNotes;
    },
    // account(profile) {
    //   return accounts.get(profile);
    // },
  });

  const addressBalances = await Promise.all(
    owners.map((owner) => bookkeeper.balance(owner.address)),
  );

  assert.equal(addressBalances[0].value, 1026179647718621n);
  assert.equal(addressBalances[1].value, 1419179830115057n);
  assert.equal(addressBalances[2].value, 512720219906168n);

  // const accountBalances = await Promise.all(
  //   owners.map((owner) => bookkeeper.balance(owner.account)),
  // );

  // assert.equal(accountBalances[0].value, 0n);
  // assert.equal(accountBalances[1].value, 0n);
  // assert.equal(accountBalances[2].value, 0n);

  await network.disconnect();
});
