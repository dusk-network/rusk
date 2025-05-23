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
  // return;
  const rawTransfers = [
    { amount: 10n, from: 0, to: 1 },
    { amount: 30n, from: 0, to: 1 },
    { amount: 20n, from: 1, to: 0 },
    { amount: 12n, from: 2, to: 1 },
  ];
  // const network = await Network.connect("http://localhost:8080/");
  const network = await Network.connect("https://testnet.nodes.dusk.network/");

  // Thomas' server
  // address: ugiaM55iFtPRSMhRRSkq5EuNZRzREx9TxfGaxZV4W4XFjayTmTMEtQrFc95qTURnFHB7rrbW4XqKQCcPG4HUU5sQ36YFmmms1y8ovjFtjWTuW645Asn8v25adkaDQoh8bzE
  // const network = await Network.connect("http://178.128.240.93:8080/");

  // trial suffer emerge awesome diary rule soft raven reason unique nasty into
  // const seeder = () =>
  //   Uint8Array.from([
  //     170, 40, 121, 200, 239, 13, 83, 6, 91, 152, 190, 48, 58, 98, 38, 225, 12,
  //     173, 189, 7, 207, 12, 79, 173, 102, 215, 3, 80, 228, 27, 218, 124, 64,
  //     212, 72, 255, 5, 5, 132, 237, 142, 59, 7, 77, 32, 178, 204, 174, 153, 146,
  //     226, 44, 249, 63, 180, 212, 68, 140, 95, 138, 117, 122, 126, 132,
  //   ]);

  // inmate coil they chicken memory light rigid tennis hip will what moon
  // const seeder = () =>
  //   Uint8Array.from([
  //     141, 85, 210, 11, 219, 130, 17, 57, 125, 24, 160, 190, 195, 59, 227, 207,
  //     236, 182, 203, 177, 170, 42, 247, 27, 51, 73, 87, 245, 249, 190, 5, 110,
  //     253, 92, 215, 41, 249, 222, 31, 139, 7, 117, 162, 175, 153, 38, 112, 227,
  //     78, 74, 129, 140, 146, 0, 70, 67, 228, 217, 117, 221, 168, 186, 197, 248,
  //   ]);

  const profileGenerator = new ProfileGenerator(seeder);
  const profiles = await Promise.all([
    profileGenerator.default,
    // profileGenerator.next(),
    // profileGenerator.next(),
  ]);

  const syncer = new AccountSyncer(network);
  // const balances = await syncer.balances(profiles);
  // const nonces = balances.map((balance) => balance.nonce);
  // const transfers = await Promise.all(
  //   rawTransfers.map((raw) =>
  //     new Transfer(profiles[raw.from])
  //       .amount(raw.amount)
  //       .to(profiles[raw.to].account)
  //       .nonce(nonces[raw.from]++)
  //       .chain(Network.LOCALNET)
  //       .gas({ limit: 500_000_000n })
  //       .build()
  //   )
  // );

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

  const streams = await syncer.history(profiles, {
    // limit: 10,
    // from: 9000n,
    // to: 11000n,
    // from: 105000n,
    // to: 113000n,
    // limit: 20,
    // limit: 5,
    order: "desc",
  });
  const txs = await Promise.all(streams.map(getTxsFromStream));

  console.log(`\ntxs: ${txs.map((tx) => tx.length)}`);
  console.log(txs.map((tx) => tx.map((tx) => tx.blockHeight)));
  // txs.map((tx) => tx.map(console.log));

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
