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
} from "@dusk/w3sper";

import { test, assert, seeder, Treasury } from "./harness.js";

test("Account Balance", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const user =
    "oCqYsUMRqpRn2kSabH52Gt6FQCwH5JXj5MtRdYVtjMSJ73AFvdbPf98p3gz98fQwNy9ZBiDem6m9BivzURKFSKLYWP3N9JahSPZs9PnZ996P18rTGAjQTNFsxtbrKx79yWu";

  const [balance] = await new AccountSyncer(network).balances([user]);

  assert.equal(balance, {
    nonce: 0n,
    value: 1_001_000_000_000_000n,
  });

  await network.disconnect();
});

test("Balances synchronization", async () => {
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

  assert.equal(addressBalances[0].value, 1_026_179_647_718_621n);
  assert.equal(addressBalances[1].value, 1_419_179_830_115_057n);
  assert.equal(addressBalances[2].value, 512_720_219_906_168n);

  const accountBalances = await Promise.all(
    owners.map((owner) => bookkeeper.balance(owner.account)),
  );

  assert.equal(accountBalances[0].value, 100_1_000_000_000_000n);
  assert.equal(accountBalances[1].value, 80_800_000_000_000n);
  assert.equal(accountBalances[2].value, 60_060_000_000_000n);

  const bookentry = bookkeeper.as(await profiles.default);
  assert.equal(
    (await bookentry.info.balance("address")).value,
    1026179647718621n,
  );
  assert.equal(
    (await bookentry.info.balance("account")).value,
    1_001_000_000_000_000n,
  );

  await network.disconnect();
});
