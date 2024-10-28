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

import { test, assert, seeder, Treasury } from "./harness.js";

test.withLocalWasm = "release";

test("Network connection", async () => {
  const network = new Network("http://localhost:8080/");

  assert.ok(!network.connected);

  assert.equal(Object.keys(await network.node.info()), [
    "bootstrappingNodes",
    "chainId",
    "kadcastAddress",
    "version",
    "versionBuild",
  ]);
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

  const treasury = new Treasury(owners);

  await treasury.update({ from: 0n, addresses, accounts });

  const bookkeeper = new Bookkeeper(treasury);

  assert.equal(iterationOwnedCountTotal, 1857);

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
