// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  test,
  assert,
} from "http://rawcdn.githack.com/mio-mini/test-harness/0.1.0/mod.js";

import { Network, ProfileGenerator } from "../src/mod.js";

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
  const syncer = await network.sync({ from: 0n });

  assert.equal(syncer.from, 0n);

  let iterationOwnedCountTotal = 0;
  syncer.addEventListener("synciteration", ({ detail }) => {
    const { ownedCount } = detail;

    iterationOwnedCountTotal += ownedCount;
  });

  const profiles = new ProfileGenerator(seeder);

  let owner1 = await profiles.default;
  let owner2 = await profiles.next();
  let owner3 = await profiles.next();

  const owners = [owner1, owner2, owner3];

  const entries = await syncer.entriesFor(owners);

  let allEntries = new Map();
  for await (let [owned, syncInfo] of entries) {
    allEntries = new Map([...allEntries, ...owned]);
  }

  assert.ok(iterationOwnedCountTotal, 1857);
  assert.ok(allEntries.size, iterationOwnedCountTotal);

  assert.ok(
    (await owner1.balance("address", allEntries)).value,
    1026179647718621n,
  );
  assert.ok(
    (await owner2.balance("address", allEntries)).value,
    1419179830115057n,
  );
  assert.ok(
    (await owner3.balance("address", allEntries)).value,
    512720219906168n,
  );

  await network.disconnect();
});
