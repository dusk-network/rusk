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

import { ProfileGenerator, Bookkeeper } from "../src/mod.js";

// Define a seed for deterministic profile generation
const SEED = new Uint8Array(64).fill(1);
const seeder = async () => SEED;

const NOTES_RKYV = "./tests/assets/notes.rkyv";
const WASM_PATH = "../target/wasm32-unknown-unknown/debug/wallet_core.wasm";

const notesBuffer = await Deno.readFile(NOTES_RKYV);

const wasm = await Deno.readFile(WASM_PATH);

import * as ProtocolDriver from "../src/protocol-driver.js";

// Test case for default profile
test("accounts into raw", async () => {
  ProtocolDriver.load(
    wasm,
    new URL("./assets/debug-imports.js", import.meta.url),
  );

  const profiles = new ProfileGenerator(seeder);

  const owners = await Promise.all([
    profiles.default,
    profiles.next(),
    profiles.next(),
  ]);

  const rawAccounts = await ProtocolDriver.accountsIntoRaw(owners);

  console.log(rawAccounts);
  await ProtocolDriver.unload();
});
