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

const resolveAfter = (delay, value) =>
  new Promise((resolve) => {
    setTimeout(() => resolve(value), delay);
  });

const skip = () => {};

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

skip("debug", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const profileGenerator = new ProfileGenerator(seeder);
  const profiles = [
    await profileGenerator.default,
    await profileGenerator.next(),
    await profileGenerator.next(),
  ];
  const syncer = new AccountSyncer(network);

  const txs = await getTxsFromStream(await syncer.history(profiles));

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

  for (const transfer of transfers) {
    await network.execute(transfer);
    await network.transactions.withId(transfer.hash).once.executed();
  }

  console.log("start waiting", new Date());

  await resolveAfter(15000);

  console.log("end waiting", new Date());

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
