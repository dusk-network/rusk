// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  AccountSyncer,
  Bookkeeper,
  Network,
  ProfileGenerator,
} from "@dusk/w3sper";

import { assert, seeder, test, Treasury } from "./harness.js";

const MINIMUM_STAKE = 1_000_000_000_000n;
const STAKE_AMOUNT = MINIMUM_STAKE + 321n;

test("minimum stake correct", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const bookkeeper = new Bookkeeper();
  const minimumStake = await bookkeeper.minimumStake;

  assert.equal(minimumStake, MINIMUM_STAKE);

  await network.disconnect();
});

test("stake amount insufficient", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookentry = new Bookkeeper(treasury).as(users[0]);

  const transfer = bookentry
    .stake(MINIMUM_STAKE - 1n)
    .gas({ limit: 500_000_000n });

  await assert.reject(
    async () => await network.execute(transfer),
    RangeError,
    `Stake amount must be greater or equal than ${MINIMUM_STAKE}`,
  );

  await network.disconnect();
});

test("cannot top up with no stake", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookentry = new Bookkeeper(treasury).as(users[1]);

  const transfer = bookentry.topup(STAKE_AMOUNT).gas({
    limit: 500_000_000n,
  });

  await assert.reject(
    async () => await network.execute(transfer),
    Error,
    "No stake to topup. Use `stake` to create a new stake",
  );

  await network.disconnect();
});

test("partial unstake insufficient", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(users[0]);

  await treasury.update({ accounts });

  const stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 0n);
  assert.equal(stakeInfo.amount.locked, 0n);
  assert.equal(stakeInfo.amount.value, MINIMUM_STAKE * 2n);

  const partialAmount = stakeInfo.amount.value - 1n;
  const transfer = bookentry.unstake(partialAmount).gas({
    limit: 500_000_000n,
  });

  await assert.reject(
    async () => await network.execute(transfer),
    RangeError,
    `Remaining stake must be greater or equal than ${MINIMUM_STAKE}`,
  );

  await network.disconnect();
});

test("stake", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];
  const treasury = new Treasury(users);

  const accounts = new AccountSyncer(network);

  await treasury.update({ accounts });

  const bookentry = new Bookkeeper(treasury).as(users[1]);

  const accountBalance = await bookentry.info.balance("account");

  let stakeInfo = await bookentry.info.stake();
  const hasNoStake = stakeInfo.amount === null;
  assert.ok(hasNoStake, "User should not have stake");

  const transfer = bookentry.stake(STAKE_AMOUNT).gas({
    limit: 500_000_000n,
  });

  const { hash } = await network.execute(transfer);

  const evt = await network.transactions.withId(hash).once.executed();
  const { gasPaid } = evt;

  await treasury.update({ accounts });

  stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount?.value, transfer.attributes.amount);

  const newAccountBalance = await bookentry.info.balance("account");

  assert.equal(
    newAccountBalance.value,
    accountBalance.value - transfer.attributes.amount - gasPaid,
  );

  await network.disconnect();
});

test("cannot stake twice", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookentry = new Bookkeeper(treasury).as(users[1]);

  const transfer = bookentry.stake(STAKE_AMOUNT).gas({
    limit: 500_000_000n,
  });

  await assert.reject(
    async () => await network.execute(transfer),
    Error,
    "Stake already exists. Use `topup` to add to the current stake",
  );

  await network.disconnect();
});

test("partial unstake", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookentry = new Bookkeeper(treasury).as(users[1]);

  await treasury.update({ accounts });

  const accountBalance = await bookentry.info.balance("account");
  let stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 4320n);
  assert.equal(stakeInfo.amount.locked, 0n);
  assert.equal(stakeInfo.amount.value, STAKE_AMOUNT);

  const transfer = bookentry.unstake(123n).gas({
    limit: 500_000_000n,
  });

  const { hash } = await network.execute(transfer);

  const evt = await network.transactions.withId(hash).once.executed();
  const { gasPaid } = evt;

  await treasury.update({ accounts });

  stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 4320n);
  assert.equal(stakeInfo.amount.locked, 0n);
  assert.equal(stakeInfo.amount.value, STAKE_AMOUNT - 123n);

  const newAccountBalance = await bookentry.info.balance("account");

  assert.equal(newAccountBalance.value, accountBalance.value + 123n - gasPaid);

  await network.disconnect();
});

test("topup with no penalty", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookentry = new Bookkeeper(treasury).as(users[1]);

  await treasury.update({ accounts });

  const accountBalance = await bookentry.info.balance("account");
  let stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 4320n);
  assert.equal(stakeInfo.amount.locked, 0n);
  assert.equal(stakeInfo.amount.value, STAKE_AMOUNT - 123n);

  const transfer = bookentry.topup(123n).gas({
    limit: 500_000_000n,
  });

  const { hash } = await network.execute(transfer);

  const evt = await network.transactions.withId(hash).once.executed();
  const { gasPaid } = evt;

  await treasury.update({ accounts });

  stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 4320n);
  assert.equal(stakeInfo.amount.locked, 0n);
  assert.equal(stakeInfo.amount.value, STAKE_AMOUNT);

  const newAccountBalance = await bookentry.info.balance("account");

  assert.equal(newAccountBalance.value, accountBalance.value - 123n - gasPaid);

  await network.disconnect();
});

test("topup with penalty", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookentry = new Bookkeeper(treasury).as(users[0]);

  await treasury.update({ accounts });

  const accountBalance = await bookentry.info.balance("account");
  let stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 0n);
  assert.equal(stakeInfo.amount.locked, 0n);
  assert.equal(stakeInfo.amount.value, MINIMUM_STAKE * 2n);

  const transfer = bookentry.topup(500n).gas({
    limit: 500_000_000n,
  });

  const { hash } = await network.execute(transfer);

  const evt = await network.transactions.withId(hash).once.executed();
  const { gasPaid } = evt;

  await treasury.update({ accounts });

  stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 0n);
  assert.equal(stakeInfo.amount.locked, 50n);
  assert.equal(stakeInfo.amount.value, MINIMUM_STAKE * 2n + 450n);

  const newAccountBalance = await bookentry.info.balance("account");

  assert.equal(newAccountBalance.value, accountBalance.value - 500n - gasPaid);

  await network.disconnect();
});

test("unstake", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(users[1]);

  await treasury.update({ accounts });

  const accountBalance = await bookentry.info.balance("account");
  let stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount.eligibility, 4320n);
  assert.equal(stakeInfo.amount.locked, 0n);
  assert.equal(stakeInfo.amount.value, STAKE_AMOUNT);

  const transfer = bookentry.unstake().gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  const evt = await network.transactions.withId(hash).once.executed();
  const { gasPaid } = evt;

  await treasury.update({ accounts });

  stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount, null);

  const newAccountBalance = await bookentry.info.balance("account");

  assert.equal(
    newAccountBalance.value,
    accountBalance.value + STAKE_AMOUNT - gasPaid,
  );

  await network.disconnect();
});

test("withdraw stake reward with no stake", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [
    await profiles.default,
    await profiles.next(),
    await profiles.next(),
  ];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(users[2]);

  await treasury.update({ accounts });

  const stakeInfo = await bookentry.info.stake();

  assert.equal(stakeInfo.amount, null);

  const transfer = bookentry.withdraw(1000n).gas({ limit: 500_000_000n });

  assert.reject(async () => await network.execute(transfer));

  await network.disconnect();
});

test("withdraw stake reward greater than available", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [
    await profiles.default,
    await profiles.next(),
    await profiles.next(),
  ];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(users[0]);

  await treasury.update({ accounts });

  const stakeInfo = await bookentry.info.stake();

  const transfer = bookentry
    .withdraw(stakeInfo.reward + 1n)
    .gas({ limit: 500_000_000n });

  await assert.reject(
    async () => await network.execute(transfer),
    RangeError,
    "The withdrawn reward amount must be less or equal",
  );

  await network.disconnect();
});

test("withdraw partial stake reward", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(users[0]);

  await treasury.update({ accounts });

  let stakeInfo = await bookentry.info.stake();
  const accountBalance = await bookentry.info.balance("account");

  const claimAmount = stakeInfo.reward / 2n;

  const transfer = bookentry.withdraw(claimAmount).gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  const evt = await network.transactions.withId(hash).once.executed();
  const { gasPaid } = evt;

  await treasury.update({ accounts });

  stakeInfo = await bookentry.info.stake();
  const newAccountBalance = await bookentry.info.balance("account");

  assert.equal(
    newAccountBalance.value,
    accountBalance.value + claimAmount - gasPaid,
  );

  await network.disconnect();
});

test("withdraw full stake reward", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(users[0]);

  await treasury.update({ accounts });

  let stakeInfo = await bookentry.info.stake();
  const accountBalance = await bookentry.info.balance("account");

  const claimAmount = stakeInfo.reward;

  const transfer = bookentry.withdraw(claimAmount).gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);

  const evt = await network.transactions.withId(hash).once.executed();
  const { gasPaid } = evt;

  await treasury.update({ accounts });

  stakeInfo = await bookentry.info.stake();
  const newAccountBalance = await bookentry.info.balance("account");

  assert.equal(
    newAccountBalance.value,
    accountBalance.value + claimAmount - gasPaid,
  );

  await network.disconnect();
});

test("withdraw 0 as stake reward", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);

  const users = [await profiles.default, await profiles.next()];

  const accounts = new AccountSyncer(network);

  const treasury = new Treasury(users);
  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(users[0]);

  await treasury.update({ accounts });

  const transfer = bookentry.withdraw(0n).gas({ limit: 500_000_000n });

  await assert.reject(
    async () => await network.execute(transfer),
    RangeError,
    "Can't withdraw an empty reward amount.",
  );

  await network.disconnect();
});
