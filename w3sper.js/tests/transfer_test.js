// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  AccountSyncer,
  AddressSyncer,
  Bookkeeper,
  Bookmark,
  Network,
  ProfileGenerator,
  Transfer,
  useAsProtocolDriver,
} from "@dusk/w3sper";

import {
  assert,
  getLocalWasmBuffer,
  seeder,
  test,
  Treasury,
} from "./harness.js";

async function getTxsFromStream(txsStream) {
  const result = [];

  for await (const tx of txsStream) {
    result.push(tx);
  }

  return result;
}

// As tests run in parellel, we can't put our history tests in
// a separate file, so we add them to the bottom of this one
// where we know which transfers are made.
// This Map is to collect the performed transfers to be checked
// later on in history tests.
const collectedTransfers = new Map();

function collectTransfer(key, direction, hash, value) {
  if (!collectedTransfers.has(key)) {
    collectedTransfers.set(key, []);
  }

  collectedTransfers.get(key).push({ direction, hash, value });
}

test("Offline account transfers", async () => {
  // Since the tests files runs in parallel, there is no guarantee that the
  // `nonce` starts from `0`, so we need to fetch the current nonce for the
  // sender from the network before the offline operations.

  const network = await Network.connect("http://localhost:8080/");

  // profile #1
  const from =
    "ocXXBAafr7xFqQTpC1vfdSYdHMXerbPCED2apyUVpLjkuycsizDxwA6b9D7UW91kG58PFKqm9U9NmY9VSwufUFL5rVRSnFSYxbiKK658TF6XjHsHGBzavFJcxAzjjBRM4eF";

  // default profile
  const to =
    "oCqYsUMRqpRn2kSabH52Gt6FQCwH5JXj5MtRdYVtjMSJ73AFvdbPf98p3gz98fQwNy9ZBiDem6m9BivzURKFSKLYWP3N9JahSPZs9PnZ996P18rTGAjQTNFsxtbrKx79yWu";

  const [balance] = await new AccountSyncer(network).balances([from]);

  // here we can disconnect from the network, since we are going to do
  // everything offline
  await network.disconnect();

  // What is inside this block, uses a local protocol driver instead of fetching
  // from the network, so it does not need to be connected.
  // All transactions are signed locally.
  const offlineOperations = useAsProtocolDriver(
    await getLocalWasmBuffer()
  ).then(async () => {
    const profiles = new ProfileGenerator(seeder);
    const users = await Promise.all([profiles.default, profiles.next()]);
    const transfers = await Promise.all(
      [77n, 22n].map((amount, nonce) =>
        new Transfer(users[1])
          .amount(amount)
          .to(to)
          .nonce(balance.nonce + BigInt(nonce))
          .chain(Network.LOCALNET)
          .gas({ limit: 500_000_000n })
          .build()
      )
    );

    assert.equal(transfers[0].nonce, balance.nonce + 1n);
    assert.equal(transfers[1].nonce, balance.nonce + 2n);

    return { transfers, users };
  });

  const { transfers, users } = await offlineOperations;

  // Here we gather the transactions generated "offline", we connect to the network,
  // and propagate all of them
  await network.connect();

  const balances = await new AccountSyncer(network).balances(users);

  let { hash } = await network.execute(transfers[0]);
  let evt = await network.transactions.withId(hash).once.executed();
  let gasPaid = evt.gasPaid;

  collectTransfer(from, "out", hash, 77n);
  collectTransfer(to, "in", hash, 77n);

  ({ hash } = await network.execute(transfers[1]));

  collectTransfer(from, "out", hash, 22n);
  collectTransfer(to, "in", hash, 22n);

  evt = await network.transactions.withId(hash).once.executed();
  gasPaid += evt.gasPaid;

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
  const { gasPaid } = await network.transactions.withId(hash).once.executed();

  collectTransfer(users[1].account.toString(), "out", hash, 77n);
  collectTransfer(users[0].account.toString(), "in", hash, 77n);

  await treasury.update({ accounts });

  const newBalances = [
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
  const from = Bookmark.from(0n);

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
  const { gasPaid } = await network.transactions.withId(hash).once.executed();

  collectTransfer(users[1].address.toString(), "out", hash, 11n);
  collectTransfer(users[0].address.toString(), "in", hash, 11n);

  await treasury.update({ addresses });

  const newBalances = [
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
  const defaultProfile = await profiles.default;

  const accounts = new AccountSyncer(network);
  const addresses = new AddressSyncer(network);

  const treasury = new Treasury([defaultProfile]);

  await treasury.update({ accounts, addresses });

  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(defaultProfile);

  const accountBalance = await bookentry.info.balance("account");
  const addressBalance = await bookentry.info.balance("address");

  const transfer = bookentry.unshield(123n).gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);
  const { gasPaid } = await network.transactions.withId(hash).once.executed();

  collectTransfer(defaultProfile.account.toString(), "out", hash, 123n);
  collectTransfer(defaultProfile.address.toString(), "in", hash, 123n);

  await treasury.update({ accounts, addresses });

  const newAccountBalance = await bookentry.info.balance("account");
  const newAddressBalance = await bookentry.info.balance("address");

  assert.equal(newAccountBalance.value, accountBalance.value + 123n);
  assert.equal(newAddressBalance.value, addressBalance.value - 123n - gasPaid);

  await network.disconnect();
});

test("shield", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);
  const defaultProfile = await profiles.default;

  const accounts = new AccountSyncer(network);
  const addresses = new AddressSyncer(network);

  const treasury = new Treasury([defaultProfile]);

  await treasury.update({ accounts, addresses });

  const bookkeeper = new Bookkeeper(treasury);
  const bookentry = bookkeeper.as(defaultProfile);

  const accountBalance = await bookentry.info.balance("account");
  const addressBalance = await bookentry.info.balance("address");

  const transfer = bookentry.shield(321n).gas({ limit: 500_000_000n });

  const { hash } = await network.execute(transfer);
  const { gasPaid } = await network.transactions.withId(hash).once.executed();

  collectTransfer(defaultProfile.address.toString(), "out", hash, 321n);
  collectTransfer(defaultProfile.account.toString(), "in", hash, 321n);

  await treasury.update({ accounts, addresses });

  const newAccountBalance = await bookentry.info.balance("account");
  const newAddressBalance = await bookentry.info.balance("address");

  assert.equal(newAccountBalance.value, accountBalance.value - 321n - gasPaid);
  assert.equal(newAddressBalance.value, addressBalance.value + 321n);

  await network.disconnect();
});

test("account memo transfer", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);
  const users = await Promise.all([profiles.default, profiles.next()]);
  const accounts = new AccountSyncer(network);
  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookkeeper = new Bookkeeper(treasury);

  let transfer = bookkeeper
    .as(users[1])
    .transfer(1n)
    .to(users[0].account)
    .memo(new Uint8Array([2, 4, 8, 16]))
    .gas({ limit: 500_000_000n });

  let { hash } = await network.execute(transfer);

  let evt = await network.transactions.withId(hash).once.executed();

  collectTransfer(users[1].account.toString(), "out", hash, 1n);
  collectTransfer(users[0].account.toString(), "in", hash, 1n);

  assert.equal([...evt.memo()], [2, 4, 8, 16]);

  await treasury.update({ accounts });

  transfer = bookkeeper
    .as(users[1])
    .transfer(1n)
    .to(users[0].account)
    .memo("Tarapia Tapioco, come fosse stringa")
    .gas({ limit: 500_000_000n });

  ({ hash } = await network.execute(transfer));

  evt = await network.transactions.withId(hash).once.executed();

  collectTransfer(users[1].account.toString(), "out", hash, 1n);
  collectTransfer(users[0].account.toString(), "in", hash, 1n);

  // deno-fmt-ignore
  assert.equal(
    [...evt.memo()],
    [
      84, 97, 114, 97, 112, 105, 97, 32, 84, 97, 112, 105, 111, 99, 111, 44, 32,
      99, 111, 109, 101, 32, 102, 111, 115, 115, 101, 32, 115, 116, 114, 105,
      110, 103, 97,
    ]
  );

  assert.equal(
    evt.memo({ as: "string" }),
    "Tarapia Tapioco, come fosse stringa"
  );

  await network.disconnect();
});

test("address memo transfer", async () => {
  const { cleanup } = useAsProtocolDriver(await getLocalWasmBuffer()); // Temporarily needed, while the node doesn't serve the latest WASM.
  const network = await Network.connect("http://localhost:8080/");
  const profiles = new ProfileGenerator(seeder);
  const users = await Promise.all([profiles.default, profiles.next()]);
  const addresses = new AddressSyncer(network);
  const treasury = new Treasury(users);

  await treasury.update({ addresses });

  const bookkeeper = new Bookkeeper(treasury);

  let transfer = bookkeeper
    .as(users[1])
    .transfer(1n)
    .to(users[0].address)
    .memo(new Uint8Array([2, 4, 8, 16]))
    .gas({ limit: 500_000_000n });

  let { hash } = await network.execute(transfer);

  let evt = await network.transactions.withId(hash).once.executed();

  collectTransfer(users[1].address.toString(), "out", hash, 1n);
  collectTransfer(users[0].address.toString(), "in", hash, 1n);

  assert.equal([...evt.memo()], [2, 4, 8, 16]);

  await treasury.update({ addresses });

  transfer = bookkeeper
    .as(users[1])
    .transfer(1n)
    .to(users[0].address)
    .memo("Tarapia Tapioco, come fosse stringa")
    .gas({ limit: 500_000_000n });

  ({ hash } = await network.execute(transfer));

  evt = await network.transactions.withId(hash).once.executed();

  collectTransfer(users[1].address.toString(), "out", hash, 1n);
  collectTransfer(users[0].address.toString(), "in", hash, 1n);

  // deno-fmt-ignore
  assert.equal(
    [...evt.memo()],
    [
      84, 97, 114, 97, 112, 105, 97, 32, 84, 97, 112, 105, 111, 99, 111, 44, 32,
      99, 111, 109, 101, 32, 102, 111, 115, 115, 101, 32, 115, 116, 114, 105,
      110, 103, 97,
    ]
  );

  assert.equal(
    evt.memo({ as: "string" }),
    "Tarapia Tapioco, come fosse stringa"
  );

  await network.disconnect();
  await cleanup(); // Remove when useAsProtocolDriver is removed.
});

test("account transfers history", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const profileGenerator = new ProfileGenerator(seeder);
  const profiles = await Promise.all([
    profileGenerator.default,
    profileGenerator.next(),
  ]);
  const syncer = new AccountSyncer(network);

  let streams = await syncer.history(profiles, { order: "desc" });
  let txs = await Promise.all(streams.map(getTxsFromStream));

  profiles.forEach((profile, idx) => {
    const key = profile.account.toString();
    const collected = collectedTransfers.get(key).toReversed();

    assert.ok(txs[idx].length > 0);
    assert.equal(txs[idx].length, collected.length);

    for (let i = 0; i < txs[idx].length; i++) {
      // TODO wait for event parsing
      // assert.equal(collected[i].direction, txs[idx][i].direction);
      assert.equal(collected[i].hash, txs[idx][i].hash);
      // TODO wait for event parsing
      // assert.equal(collected[i].value, txs[idx][i].value);
    }
  });

  streams = await syncer.history(profiles, { limit: 1 });
  txs = await Promise.all(streams.map(getTxsFromStream));

  profiles.forEach((profile, idx) => {
    const key = profile.account.toString();
    const collected = collectedTransfers.get(key);

    assert.equal(txs[idx].length, 1);

    // TODO wait for event parsing
    // assert.equal(collected[0].direction, txs[idx][0].direction);
    assert.equal(collected[0].hash, txs[idx][0].hash);
    // TODO wait for event parsing
    // assert.equal(collected[0].value, txs[idx][0].value);
  });

  await network.disconnect();
});
