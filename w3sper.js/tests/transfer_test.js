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
  NETWORK,
  seeder,
  test,
  Treasury,
} from "./harness.js";

const getBlockHashByHeight = (network, height) =>
  network
    .query(`block(height: ${height}) { header { hash } }`)
    .then((data) => data.block.header.hash);

async function waitForFinalizedBlock(network, blockHeight) {
  const blockHash = await getBlockHashByHeight(network, blockHeight);
  const alreadyFinalized = (
    await network.query(`checkBlock(
      height: ${blockHeight},
      hash: "${blockHash}",
      onlyFinalized: true
    )`)
  ).checkBlock;

  if (alreadyFinalized === true) {
    return;
  }

  const { promise, reject, resolve } = Promise.withResolvers();
  const controller = new AbortController();
  const timeoutSignal = AbortSignal.timeout(20_000);
  const onAbort = () => {
    controller.abort();
    reject(new Error(`Timeout: block ${blockHash} not finalized after 20s`));
  };

  timeoutSignal.addEventListener("abort", onAbort, { once: true });

  network.blocks.withId(blockHash).on.statechange(
    ({ payload }) => {
      if (payload.state === "finalized") {
        timeoutSignal.removeEventListener("abort", onAbort);
        controller.abort();
        resolve();
      }
    },
    { signal: controller.signal }
  );

  return promise;
}

async function getTxsFromStream(txsStream) {
  const result = [];

  for await (const tx of txsStream) {
    result.push(tx);
  }

  return result;
}

/**
 * This Map is to collect the transfers we make
 * in these tests, so that we can check them later
 * in our history tests at the bottom of this file.
 */
const collectedTransfers = new Map();

/**
 * @param {string} key
 * @param {Object} info
 * @param {string} info.from
 * @param {string} info.hash
 * @param {string} info.method
 * @param {string} info.to
 * @param {bigint} info.value
 */
function collectTransfer(key, info) {
  if (!collectedTransfers.has(key)) {
    collectedTransfers.set(key, []);
  }

  collectedTransfers.get(key).push(info);
}

let HISTORY_FROM = 0n;

test("Setting up `HISTORY_FROM` constant", async () => {
  const network = await Network.connect(NETWORK);
  const currentBlockHeight = await network.blockHeight;

  await waitForFinalizedBlock(network, currentBlockHeight);

  /**
   * We create a HISTORY_FROM constant to exclude from
   * out transaction history test the transfers that happened
   * in other files.
   */
  HISTORY_FROM = currentBlockHeight + 1n;

  assert.ok(HISTORY_FROM > 1n);

  await network.disconnect();
});

test("Offline account transfers", async () => {
  // Since the tests files runs in parallel, there is no guarantee that the
  // `nonce` starts from `0`, so we need to fetch the current nonce for the
  // sender from the network before the offline operations.

  const network = await Network.connect(NETWORK);

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

  collectTransfer(from, { from, method: "transfer", to, hash, value: -77n });
  collectTransfer(to, { from, method: "transfer", to, hash, value: 77n });

  ({ hash } = await network.execute(transfers[1]));

  collectTransfer(from, { from, method: "transfer", to, hash, value: -22n });
  collectTransfer(to, { from, method: "transfer", to, hash, value: 22n });

  evt = await network.transactions.withId(hash).once.executed();
  gasPaid += evt.gasPaid;

  const newBalances = await new AccountSyncer(network).balances(users);

  assert.equal(newBalances[0].value, balances[0].value + 77n + 22n);
  assert.equal(newBalances[1].nonce, balances[1].nonce + 2n);
  assert.equal(newBalances[1].value, balances[1].value - 77n - 22n - gasPaid);

  await network.disconnect();
});

test("accounts", async () => {
  const network = await Network.connect(NETWORK);
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
  const baseInfo = {
    from: users[1].account.toString(),
    hash,
    method: "transfer",
    to: users[0].account.toString(),
  };

  collectTransfer(baseInfo.from, { ...baseInfo, value: -77n });
  collectTransfer(baseInfo.to, { ...baseInfo, value: 77n });

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
  const network = await Network.connect(NETWORK);
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
  const baseInfo = {
    from: "Phoenix",
    hash,
    method: "transfer",
    to: "Phoenix",
  };

  collectTransfer(users[1].address.toString(), { ...baseInfo, value: 11n });
  collectTransfer(users[0].address.toString(), { ...baseInfo, value: -11n });

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
  const network = await Network.connect(NETWORK);
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
  const baseInfo = {
    from: "Phoenix",
    hash,
    method: "convert",
    to: defaultProfile.account.toString(),
  };

  collectTransfer(baseInfo.to, { ...baseInfo, value: 123n });
  collectTransfer(defaultProfile.address.toString(), {
    ...baseInfo,
    value: -123n,
  });

  await treasury.update({ accounts, addresses });

  const newAccountBalance = await bookentry.info.balance("account");
  const newAddressBalance = await bookentry.info.balance("address");

  assert.equal(newAccountBalance.value, accountBalance.value + 123n);
  assert.equal(newAddressBalance.value, addressBalance.value - 123n - gasPaid);

  await network.disconnect();
});

test("shield", async () => {
  const network = await Network.connect(NETWORK);
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
  const baseInfo = {
    from: defaultProfile.account.toString(),
    hash,
    method: "convert",
    to: "Phoenix",
  };

  collectTransfer(baseInfo.from, { ...baseInfo, value: -321n });
  collectTransfer(defaultProfile.address.toString(), {
    ...baseInfo,
    value: 321n,
  });

  await treasury.update({ accounts, addresses });

  const newAccountBalance = await bookentry.info.balance("account");
  const newAddressBalance = await bookentry.info.balance("address");

  assert.equal(newAccountBalance.value, accountBalance.value - 321n - gasPaid);
  assert.equal(newAddressBalance.value, addressBalance.value + 321n);

  await network.disconnect();
});

test("account memo transfer", async () => {
  const network = await Network.connect(NETWORK);
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

  const baseInfo = {
    from: users[1].account.toString(),
    hash,
    method: "transfer",
    to: users[0].account.toString(),
  };

  collectTransfer(baseInfo.from, { ...baseInfo, value: -1n });
  collectTransfer(baseInfo.to, { ...baseInfo, value: 1n });

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

  baseInfo.hash = hash;

  collectTransfer(baseInfo.from, { ...baseInfo, value: -1n });
  collectTransfer(baseInfo.to, { ...baseInfo, value: 1n });

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
  const network = await Network.connect(NETWORK);
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

  const baseInfo = {
    from: "Phoenix",
    hash,
    method: "transfer",
    to: "Phoenix",
  };

  collectTransfer(users[1].address.toString(), { ...baseInfo, value: -1n });
  collectTransfer(users[0].address.toString(), { ...baseInfo, value: 1n });

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

  baseInfo.hash = hash;

  collectTransfer(users[1].address.toString(), { ...baseInfo, value: -1n });
  collectTransfer(users[0].address.toString(), { ...baseInfo, value: 1n });

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

test("address memo transfer using payload method", async () => {
  const network = await Network.connect(NETWORK);
  const profiles = new ProfileGenerator(seeder);
  const users = await Promise.all([profiles.default, profiles.next()]);
  const addresses = new AddressSyncer(network);
  const treasury = new Treasury(users);

  await treasury.update({ addresses });

  const bookkeeper = new Bookkeeper(treasury);

  const memo_payload_1 = {
    memo: new Uint8Array([2, 4, 8, 16]),
  };

  let transfer = bookkeeper
    .as(users[1])
    .transfer(1n)
    .to(users[0].address)
    .payload(memo_payload_1)
    .gas({ limit: 500_000_000n });

  let { hash } = await network.execute(transfer);

  let evt = await network.transactions.withId(hash).once.executed();

  const baseInfo = {
    from: "Phoenix",
    hash,
    method: "get_version",
    to: "Phoenix",
  };

  collectTransfer(users[1].address.toString(), { ...baseInfo, value: -1n });
  collectTransfer(users[0].address.toString(), { ...baseInfo, value: 1n });

  assert.equal([...evt.memo()], [2, 4, 8, 16]);

  await treasury.update({ addresses });

  const memo_payload_2 = {
    memo: "Tarapia Tapioco, come fosse stringa",
  };

  transfer = bookkeeper
    .as(users[1])
    .transfer(1n)
    .to(users[0].address)
    .payload(memo_payload_2)
    .gas({ limit: 500_000_000n });

  ({ hash } = await network.execute(transfer));

  evt = await network.transactions.withId(hash).once.executed();

  baseInfo.hash = hash;

  collectTransfer(users[1].address.toString(), { ...baseInfo, value: -1n });
  collectTransfer(users[0].address.toString(), { ...baseInfo, value: 1n });

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

test("account contract call transfer", async () => {
  const GAS_LIMIT = 500_000_000n;
  const METHOD = "get_version";
  const network = await Network.connect(NETWORK);
  const profiles = new ProfileGenerator(seeder);
  const users = await Promise.all([profiles.default, profiles.next()]);
  const accounts = new AccountSyncer(network);
  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookkeeper = new Bookkeeper(treasury);

  const payload = {
    fnName: METHOD,
    fnArgs: [],
    contractId: [
      0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0,
    ],
  };

  const transfer = bookkeeper
    .as(users[1])
    .transfer(1n)
    .to(users[0].account)
    .payload(payload)
    .gas({ limit: GAS_LIMIT });

  const { hash } = await network.execute(transfer);
  const baseInfo = {
    from: users[1].account.toString(),
    hash,
    method: "get_version",
    to: users[0].account.toString(),
  };

  const evt = await network.transactions.withId(hash).once.executed();

  collectTransfer(baseInfo.from, { ...baseInfo, value: -1n });
  collectTransfer(baseInfo.to, { ...baseInfo, value: 1n });

  assert.ok(!evt.payload.err, "contract call error");
  assert.ok(evt.payload.gas_spent < GAS_LIMIT, "gas limit reached");

  const { contract, fn_name } = evt.call();

  assert.equal(
    contract,
    "0200000000000000000000000000000000000000000000000000000000000000"
  );
  assert.equal(fn_name, METHOD);

  await treasury.update({ accounts });

  await network.disconnect();
});

test("account contract call genesis with deposit", async () => {
  const GAS_LIMIT = 500_000_000n;
  const METHOD = "get_version";
  const network = await Network.connect(NETWORK);
  const profiles = new ProfileGenerator(seeder);
  const users = await Promise.all([profiles.default]);
  const accounts = new AccountSyncer(network);
  const treasury = new Treasury(users);

  await treasury.update({ accounts });

  const bookkeeper = new Bookkeeper(treasury);

  const payload = {
    fnName: METHOD,
    fnArgs: [],
    contractId: [
      0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0,
    ],
  };

  const transfer = bookkeeper
    .as(users[0])
    .transfer(0n)
    .to(users[0].account)
    .deposit(1n)
    .payload(payload)
    .gas({ limit: GAS_LIMIT });

  const { hash } = await network.execute(transfer);
  const baseInfo = {
    from: users[0].account.toString(),
    hash,
    method: "get_version",
    // History stream uses "N/A" as the 'to' for this kind of TX
    to: "N/A"
  };

  const evt = await network.transactions.withId(hash).once.executed();

  // This does not involve an account-to-account transfer, so value can be set to 0
  collectTransfer(baseInfo.from, { ...baseInfo, value: 0n });

  assert.ok(!evt.payload.err, "contract call error");
  assert.ok(evt.payload.gas_spent < GAS_LIMIT, "gas limit reached");

  const { contract, fn_name } = evt.call();

  assert.equal(
    contract,
    "0200000000000000000000000000000000000000000000000000000000000000"
  );
  assert.equal(fn_name, METHOD);

  await treasury.update({ accounts });

  await network.disconnect();
});

test("account transfers history", async () => {
  const network = await Network.connect(NETWORK);
  const profileGenerator = new ProfileGenerator(seeder);
  const profiles = await Promise.all([
    profileGenerator.default,
    profileGenerator.next(),
  ]);
  const syncer = new AccountSyncer(network);

  /**
   * We need to wait for the block of the last transfer
   * to be finalized to see it in the history.
   * So we just wait for the current block height to be finalized.
   */
  await waitForFinalizedBlock(network, await network.blockHeight);

  let streams = await syncer.history(profiles, {
    from: HISTORY_FROM,
    order: "desc",
  });
  let txs = await Promise.all(streams.map(getTxsFromStream));

  profiles.forEach((profile, idx) => {
    const key = profile.account.toString();
    const collected = collectedTransfers.get(key).toReversed();

    assert.ok(txs[idx].length > 0);
    assert.equal(txs[idx].length, collected.length);

    for (let i = 0; i < txs[idx].length; i++) {
      assert.equal(collected[i].from, txs[idx][i].from);
      assert.equal(collected[i].hash, txs[idx][i].hash);
      assert.equal(collected[i].method, txs[idx][i].method);
      assert.equal(collected[i].to, txs[idx][i].to);
      assert.equal(collected[i].value, txs[idx][i].value);
    }
  });

  streams = await syncer.history(profiles, { from: HISTORY_FROM, limit: 1 });
  txs = await Promise.all(streams.map(getTxsFromStream));

  profiles.forEach((profile, idx) => {
    const key = profile.account.toString();
    const collected = collectedTransfers.get(key);

    assert.equal(txs[idx].length, 1);
    assert.equal(collected[0].from, txs[idx][0].from);
    assert.equal(collected[0].hash, txs[idx][0].hash);
    assert.equal(collected[0].method, txs[idx][0].method);
    assert.equal(collected[0].to, txs[idx][0].to);
    assert.equal(collected[0].value, txs[idx][0].value);
  });

  const noTransfersProfile = await profileGenerator.next();

  streams = await syncer.history([noTransfersProfile], { from: HISTORY_FROM });
  txs = await Promise.all(streams.map(getTxsFromStream));

  assert.equal(txs[0].length, 0);

  await network.disconnect();
});
