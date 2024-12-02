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

const rejectAfter = (delay, error) =>
  new Promise((_, reject) => {
    setTimeout(() => reject(error), delay);
  });

const skip = () => {};

function waitForFinalizedBlock(network, blockHash) {
  const { promise, resolve } = Promise.withResolvers();
  const controller = new AbortController();
  console.log(`aspetto che sia finalizzato ${blockHash}`);
  network.blocks.withId(blockHash).on.statechange(
    ({ payload }) => {
      if (payload.state === "finalized") {
        console.log("finalized", payload.atHeight);
        controller.abort();
        resolve();
      }
    },
    { signal: controller.signal }
  );

  return promise;
}

skip("debug", async () => {
  const network = await Network.connect("http://localhost:8080/");
  const syncer = new AccountSyncer(network);
  const profileGenerator = new ProfileGenerator(seeder);
  const profiles = [
    await profileGenerator.default,
    await profileGenerator.next(),
    await profileGenerator.next(),
  ];

  console.log(await getTxsFromStream(syncer.history(profiles)));

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

  // await network.execute(transfers[0]);

  const syncer = new AccountSyncer(network);

  const txs = await getTxsFromStream(syncer.history([profiles[0]]));

  console.log(txs);

  await network.disconnect();
});

test("accounts history", async () => {
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

  let lastBlockHash;

  for (const transfer of transfers) {
    await network.execute(transfer);

    // ({ blockHash: lastBlockHash } = await network.transactions
    //   .withId(transfer.hash)
    //   .once.executed());

    // console.log("Impostato a", lastBlockHash);
    const evt = await network.transactions
      .withId(transfer.hash)
      .once.executed();
    // console.log(evt.blockHash, evt.payload);
    console.log(evt);
    console.log(evt.payload);
    // for (var prop in evt) console.log(`${prop}: ${evt[prop]}`);

    lastBlockHash = evt.blockHash;
    console.log("Impostato a", lastBlockHash);
  }

  await Promise.race([
    waitForFinalizedBlock(network, lastBlockHash),
    rejectAfter(
      20000,
      new Error(`Timed out waiting for block finalization (${lastBlockHash})`)
    ),
  ]);

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
