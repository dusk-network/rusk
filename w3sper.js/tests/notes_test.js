// This Source Code Form is subject to the terms of the Mozilla Public
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  test,
  assert,
} from "http://rawcdn.githack.com/mio-mini/test-harness/0.1.0/mod.js";

const hex = (bytes) =>
  Array.from(bytes)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");

import { ProfileGenerator } from "../src/mod.js";

// Define a seed for deterministic profile generation
const SEED = new Uint8Array(64).fill(1);
const seeder = async () => SEED;

const notesBuffer = await Deno.readFile("./tests/assets/notes.rkyv");

import * as ProtocolDriver from "../src/protocol-driver.js";

const wasmBuffer = await Deno.readFile(
  "../target/wasm32-unknown-unknown/release/wallet_core.wasm",
);

// Test case for default profile
test("owened notes balance", async () => {
  ProtocolDriver.load(
    wasmBuffer,
    new URL("./assets/debug-imports.js", import.meta.url),
  );

  const profiles = new ProfileGenerator(seeder);

  const owner1 = await Promise.all([
    profiles.default,
    profiles.next(),
    profiles.next(),
  ]);

  const owner2 = await Promise.all([profiles.next(), profiles.next()]);
  const owner3 = [await profiles.next()];
  const owner4 = [await profiles.next()];

  let [notes] = await ProtocolDriver.mapOwned(owner1, notesBuffer);

  assert.equal(notes.size, 14);
  assert.equal([...notes.keys()].map(hex), [
    "b3063d50864e5e138db87447e0bdaf4cf345c17dd2da0b7ac7abbc08b090e62b",
    "1afa317b0d0c8bcf2c08890380954adabb19ffd4bf721422179b6f08b664394a",
    "7a70d19b83c4722a6b27e2f5712e9ad3b7573656d32ccf847ec95d3ae942955b",
    "f628a8d5bafffe9943fa61672d00019a136efc418bbbb157df3f08bd964fc13c",
    "275f94a9b14a4b87e9b1921246a55f6095972fefb12a246554d6c276dd8a0f68",
    "e3aa198c485f2c874b8592eb1accf96bddae24787df3c4f300b660d1be97c55b",
    "e6b6da2109e2668b98a01005c90a489fa54493bc5487afe50b8ec1d5f560685d",
    "15230b3e429007eba2c85072bcef3db76f5721203f589de10d24a5c319f0386b",
    "f8a3b6b8466ccd11684398f72abb7ab5c6c1b94891eeca4e100da202a6519317",
    "11e70166fee323718a1b55a950d5701315a509043e26d5591d0a069110c12b12",
    "b76449a3ad7cc5f86b4f318c20273344291aeeb624cfb9580d960ffef4c4926d",
    "65e08a957c08c32aa955f3b5edd13ead24f8d63408679cae381d1f6e2412e914",
    "3abfda24337c08c8da2cb28369a48b55916329038c1287521749cdb9f1efce02",
    "7cd032f0160e0a54ca1de4d86ee8c9981289121982c28a916752ffc06c9fc72a",
  ]);

  let balance = await owner1[0].balance("address", notes);
  assert.equal(balance, { value: 67n, spendable: 39n });

  let picked = await ProtocolDriver.pickNotes(owner1[0], notes, 10n);
  assert.equal(picked.size, 4);
  assert.equal(await owner1[0].balance("address", picked), {
    value: 10n,
    spendable: 10n,
  });

  picked = await ProtocolDriver.pickNotes(owner1[0], notes, 14n);
  assert.equal(picked.size, 4);
  assert.equal(await owner1[0].balance("address", picked), {
    value: 15n,
    spendable: 15n,
  });

  [notes] = await ProtocolDriver.mapOwned(owner2, notesBuffer);
  assert.equal(notes.size, 2);
  assert.equal([...notes.keys()].map(hex), [
    "cc7a09800474fc6668fba1f1b631e940f00309d88a9424fe1c84ca93d627b518",
    "96c24f81d2587017726ec7bbcbbfa80d5f300002c5d8dabe53870e97379d873f",
  ]);

  balance = await owner2[0].balance("address", notes);
  assert.equal(balance, { value: 3n, spendable: 3n });

  [notes] = await ProtocolDriver.mapOwned(owner3, notesBuffer);
  assert.equal(notes.size, 1);
  assert.equal([...notes.keys()].map(hex), [
    "81d45c11c5e9b20c2ebba17eaa4f720c669bfd4876cf620280c296225b721c18",
  ]);

  picked = await ProtocolDriver.pickNotes(owner3[0], notes, 14n);
  assert.equal(picked.size, 1);
  assert.equal(await owner3[0].balance("address", picked), {
    value: 42n,
    spendable: 42n,
  });

  balance = await owner3[0].balance("address", notes);
  assert.equal(balance, { value: 42n, spendable: 42n });

  [notes] = await ProtocolDriver.mapOwned(owner4, notesBuffer);
  assert.equal(notes.size, 0);

  balance = await owner4[0].balance("address", notes);
  assert.equal(balance, { value: 0n, spendable: 0n });

  picked = await ProtocolDriver.pickNotes(owner4, notes, 1n);
  assert.equal(picked.size, 0);

  await ProtocolDriver.unload();
});
