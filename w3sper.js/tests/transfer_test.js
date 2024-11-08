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
  Transfer,
} from "../src/mod.js";

import { test, assert, seeder, Treasury } from "./harness.js";

test.withLocalWasm = "release";

const getGasPaid = (payload) =>
  new Gas({
    limit: payload["gas_spent"],
    price: payload.inner.fee["gas_price"],
  }).total;

test("Offline account transfers", async () => {
  const profiles = new ProfileGenerator(seeder);
  const users = await Promise.all([profiles.default, profiles.next()]);
  const to =
    "oCqYsUMRqpRn2kSabH52Gt6FQCwH5JXj5MtRdYVtjMSJ73AFvdbPf98p3gz98fQwNy9ZBiDem6m9BivzURKFSKLYWP3N9JahSPZs9PnZ996P18rTGAjQTNFsxtbrKx79yWu";

  const transfers = await Promise.all(
    [77n, 22n].map((amount, nonce) =>
      new Transfer(users[1])
        .amount(amount)
        .to(to)
        .nonce(BigInt(nonce))
        .chain(Network.LOCALNET)
        .gas({ limit: 500_000_000n })
        .build(),
    ),
  );

  assert.equal(
    transfers[0].hash,
    "72bc75e53d31afec67e32df825e5793594d937ae2c8d5b0726e833dc21db2b0d",
  );
  assert.equal(transfers[0].nonce, 1n);

  assert.equal(
    transfers[1].hash,
    "9b4039406a620b7537ab873e17c0ae5442afa4514a59f77b95644effd293936f",
  );
  assert.equal(transfers[1].nonce, 2n);

  const network = await Network.connect("http://localhost:8080/");

  const balances = await new AccountSyncer(network).balances(users);

  let { hash } = await network.execute(transfers[0]);

  let { payload } = await network.transactions.withId(hash).once.executed();
  let gasPaid = getGasPaid(payload);

  ({ hash } = await network.execute(transfers[1]));

  ({ payload } = await network.transactions.withId(hash).once.executed());
  gasPaid += getGasPaid(payload);

  const newBalances = await new AccountSyncer(network).balances(users);

  assert.equal(newBalances[0].value, balances[0].value + 77n + 22n);
  assert.equal(newBalances[1].nonce, balances[1].nonce + 2n);
  assert.equal(newBalances[1].value, balances[1].value - 77n - 22n - gasPaid);

  await network.disconnect();
});

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
    .as(users[1])
    .transfer(77n)
    .to(users[0].account)
    .gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  const { payload } = await network.transactions.withId(hash).once.executed();

  const gasPaid = getGasPaid(payload);

  await treasury.update({ accounts });

  let newBalances = [
    await bookkeeper.balance(users[0].account),
    await bookkeeper.balance(users[1].account),
  ];

  assert.equal(newBalances[0].value, balances[0].value + 77n);
  assert.equal(newBalances[1].nonce, balances[1].nonce + 1n);
  assert.equal(newBalances[1].value, balances[1].value - gasPaid - 77n);

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
    .as(users[1])
    .transfer(11n)
    .to(users[0].address)
    .gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  const { payload } = await network.transactions.withId(hash).once.executed();
  const gasPaid = getGasPaid(payload);

  await treasury.update({ addresses });

  let newBalances = [
    await bookkeeper.balance(users[0].address),
    await bookkeeper.balance(users[1].address),
  ];

  assert.equal(newBalances[0].value, balances[0].value + 11n);
  assert.equal(newBalances[1].value, balances[1].value - 11n - gasPaid);

  await network.disconnect();
});

test("unshield", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const accounts = new AccountSyncer(network);
  const addresses = new AddressSyncer(network);

  const treasury = new Treasury([await profiles.default]);

  await treasury.update({ accounts, addresses });

  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(await profiles.default);

  const accountBalance = await bookentry.balance("account");
  const addressBalance = await bookentry.balance("address");

  const transfer = bookentry.unshield(123n).gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  const { payload } = await network.transactions.withId(hash).once.executed();
  const gasPaid = getGasPaid(payload);

  await treasury.update({ accounts, addresses });

  const newAccountBalance = await bookentry.balance("account");
  const newAddressBalance = await bookentry.balance("address");

  assert.equal(newAccountBalance.value, accountBalance.value + 123n);
  assert.equal(newAddressBalance.value, addressBalance.value - 123n - gasPaid);

  await network.disconnect();
});

test("shield", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const accounts = new AccountSyncer(network);
  const addresses = new AddressSyncer(network);

  const treasury = new Treasury([await profiles.default]);

  await treasury.update({ accounts, addresses });

  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(await profiles.default);

  const accountBalance = await bookentry.balance("account");
  const addressBalance = await bookentry.balance("address");

  const transfer = bookentry.shield(321n).gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  const { payload } = await network.transactions.withId(hash).once.executed();
  const gasPaid = getGasPaid(payload);

  await treasury.update({ accounts, addresses });

  const newAccountBalance = await bookentry.balance("account");
  const newAddressBalance = await bookentry.balance("address");

  assert.equal(newAccountBalance.value, accountBalance.value - 321n - gasPaid);
  assert.equal(newAddressBalance.value, addressBalance.value + 321n);

  await network.disconnect();
});
