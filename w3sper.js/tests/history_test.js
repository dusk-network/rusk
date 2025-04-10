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

const getBlockHashByHeight = (network, height) =>
  network
    .query(`block(height: ${height}) { header { hash } }`)
    .then((data) => data.block.header.hash);

const getSortedHashes = (txs) =>
  txs
    .toSorted((a, b) => Number(b.blockHeight - a.blockHeight))
    .map((tx) => tx.id);

async function getTxsFromStream(txsStream) {
  const result = [];

  for await (const tx of txsStream) {
    result.push(tx);
  }

  return result;
}

const profileIndexer = (rawTransfers) => (profileIdx) =>
  rawTransfers.reduce((result, current, idx) => {
    if (current.from === profileIdx || current.to === profileIdx) {
      result.push(idx);
    }

    return result;
  }, []);

const skip = () => {};

function waitForFinalizedBlock(network, blockHash) {
  const { promise, resolve } = Promise.withResolvers();
  const controller = new AbortController();

  network.blocks.withId(blockHash).on.statechange(
    ({ payload }) => {
      if (payload.state === "finalized") {
        controller.abort();
        resolve();
      }
    },
    { signal: controller.signal }
  );

  return promise;
}

test("accounts history", async () => {
  const rawTransfers = [
    { amount: 10n, from: 0, to: 1 },
    { amount: 30n, from: 0, to: 1 },
    { amount: 20n, from: 1, to: 0 },
    { amount: 12n, from: 2, to: 1 },
  ];
  const network = await Network.connect("http://localhost:8080/");
  const profileGenerator = new ProfileGenerator(seeder);
  const profiles = await Promise.all([
    profileGenerator.default,
    profileGenerator.next(),
    profileGenerator.next(),
  ]);
  const syncer = new AccountSyncer(network);
  const balances = await syncer.balances(profiles);
  const nonces = balances.map((balance) => balance.nonce);
  const transfers = await Promise.all(
    rawTransfers.map((raw) =>
      new Transfer(profiles[raw.from])
        .amount(raw.amount)
        .to(profiles[raw.to].account)
        .nonce(nonces[raw.from]++)
        .chain(Network.LOCALNET)
        .gas({ limit: 500_000_000n })
        .build()
    )
  );

  // let lastBlockHeight;

  // for (const transfer of transfers) {
  //   await network.execute(transfer);

  //   const evt = await network.transactions
  //     .withId(transfer.hash)
  //     .once.executed();

  //   lastBlockHeight = evt.payload.block_height;
  // }

  // /**
  //  * We need to wait for the block of the last transfer
  //  * to be finalized to see it in the history.
  //  */
  // await waitForFinalizedBlock(
  //   network,
  //   await getBlockHashByHeight(network, lastBlockHeight)
  // );

  let txs;

  // txs = (
  //   await getTxsFromStream(syncer.history(profiles, { order: "desc" }))
  // );

  txs = await Promise.all(
    syncer.history(profiles, { order: "desc" }).map(getTxsFromStream)
  );

  // console.log(txs);
  console.log(`\ntxs: ${txs.map((tx) => tx.length)}`);
  console.log(txs.map((tx) => tx.map((tx) => tx.blockHeight)));

  await network.disconnect();
});

skip("accounts history", async () => {
  const rawTransfers = [
    { amount: 10n, from: 0, to: 1 },
    { amount: 30n, from: 0, to: 1 },
    { amount: 20n, from: 1, to: 0 },
    { amount: 12n, from: 2, to: 1 },
  ];
  const getIndexesForProfile = profileIndexer(rawTransfers);
  const { profiles, transfers } = await useAsProtocolDriver(
    await getLocalWasmBuffer()
  ).then(async () => {
    const profileGenerator = new ProfileGenerator(seeder);
    const profiles = await Promise.all([
      profileGenerator.default,
      profileGenerator.next(),
      profileGenerator.next(),
    ]);
    const nonces = Array(profiles.length).fill(0n);
    const transfers = await Promise.all(
      rawTransfers.map((raw) =>
        new Transfer(profiles[raw.from])
          .amount(raw.amount)
          .to(profiles[raw.to].account)
          .nonce(nonces[raw.from]++)
          .chain(Network.LOCALNET)
          .gas({ limit: 500_000_000n })
          .build()
      )
    );

    return { profiles, transfers };
  });

  const network = await Network.connect("http://localhost:8080/");

  let lastBlockHeight;

  for (const transfer of transfers) {
    await network.execute(transfer);

    const evt = await network.transactions
      .withId(transfer.hash)
      .once.executed();

    lastBlockHeight = evt.payload.block_height;
  }

  await waitForFinalizedBlock(
    network,
    await getBlockHashByHeight(network, lastBlockHeight)
  );

  const syncer = new AccountSyncer(network);

  let txs;

  // All transactions
  const allHashes = transfers
    .flatMap((transfer) => [transfer.hash, transfer.hash])
    .toReversed();

  txs = await getTxsFromStream(syncer.history(profiles));

  assert.equal(
    getSortedHashes(txs).join(","),
    allHashes.join(","),
    "All hashes"
  );

  // Per profile transactions
  for (let i = 0; i < profiles.length; i++) {
    const profileHashes = getIndexesForProfile(i)
      .map((idx) => transfers[idx].hash)
      .toReversed();

    txs = await getTxsFromStream(syncer.history([profiles[i]]));

    assert.equal(
      getSortedHashes(txs).join(","),
      profileHashes.join(","),
      `hashes for profile ${i}`
    );
  }

  await network.disconnect();
});
