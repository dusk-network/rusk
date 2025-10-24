// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  AccountSyncer,
  Bookkeeper,
  Contract,
  Network,
  ProfileGenerator,
} from "../src/mod.js";
import { assert, test, Treasury } from "./harness.js";
import * as bip39 from "npm:bip39";

// Generate 64 byte seed from the mnemonic.
export const seeder = async () =>
  await bip39.mnemonicToSeed(
    "eyebrow milk fan usage maximum exhibit ski nut wing sell alone great",
  );

// We'll use the Transfer and Stake contract data-drivers instead of the wallet-core FFI
const TRANSFER_ID =
  "0100000000000000000000000000000000000000000000000000000000000000";
const STAKE_ID =
  "0200000000000000000000000000000000000000000000000000000000000000";

// Built driver artifacts for stake and transfer contracts (produced by: deno task wasm)
const STAKE_WASM = "dusk_stake_contract_dd_opt.wasm";
const TRANSFER_WASM = "dusk_transfer_contract_dd_opt.wasm";

const NETWORK = "http://localhost:8080/";
const GAS_LIMIT = 500_000_000n;

// Wrapper function to grab a data-driver from the Rust release folder
async function readDriverFromTarget(name) {
  const url = new URL(
    `../../target/wasm32-unknown-unknown/release/${name}`,
    import.meta.url,
  );
  return new Uint8Array(await Deno.readFile(url));
}

test("contract.call: stake.get_version", async () => {
  const network = await Network.connect(NETWORK);
  try {
    // Add a data-driver to the local registry for easy retrieval/reuse
    network.dataDrivers.register(
      STAKE_ID,
      () => readDriverFromTarget(STAKE_WASM),
    );
    // Instantiate a new Stake contract
    const stake = new Contract({
      contractId: STAKE_ID,
      driver: network.dataDrivers.get(STAKE_ID),
      network,
    });

    // Make a call (read) to the `get_version` function on the stake contract
    const version = await stake.call.get_version();
    console.log("[version]", version);
    assert.ok(version == 8);
  } finally {
    await network.disconnect();
  }
});

test("contract.call: transfer.chain_id", async () => {
  const network = await Network.connect(NETWORK);
  try {
    network.dataDrivers.register(
      TRANSFER_ID,
      () => readDriverFromTarget(TRANSFER_WASM),
    );
    const transfer = new Contract({
      contractId: TRANSFER_ID,
      driver: network.dataDrivers.get(TRANSFER_ID),
      network,
    });

    const chainId = await transfer.call.chain_id();
    console.log("[chain_id]", chainId);
    assert.ok(chainId == 0);
  } finally {
    await network.disconnect();
  }
});

test("contract.tx/send + events.once: transfer.deposit", async () => {
  const network = await Network.connect(NETWORK);
  try {
    // Unlike the two prior calls, this is a write/transaction, thus requires a profile and sync.
    // Seed & sync
    const profiles = new ProfileGenerator(seeder);
    const users = [await profiles.default];
    const accounts = new AccountSyncer(network);
    const treasury = new Treasury(users);
    await treasury.update({ accounts });

    // Register driver, bind facade to BookEntry (driver auto-fetched as already registered prior)
    network.dataDrivers.register(
      TRANSFER_ID,
      () => readDriverFromTarget(TRANSFER_WASM),
    );
    const bookentry = new Bookkeeper(treasury).as(users[0]);
    const transfer = bookentry.contract(TRANSFER_ID, network); // driver comes from registry

    // Subscribe to decoded 'deposit' before executing
    const deposit$ = transfer.events.deposit.once();

    // Execute with matching args <-> deposit
    const AMOUNT = 2n;
    const builder = await transfer.tx.deposit(Number(AMOUNT));
    const { hash } = await network.execute(
      builder.to(users[0].account).deposit(AMOUNT).gas({ limit: GAS_LIMIT }),
    );
    assert.ok(typeof hash === "string" && hash.length > 0);

    // Check that the transaction is indeed executed
    const executed = await network.transactions.withId(hash).once.executed();
    const call = executed.call();
    assert.equal(call.contract, TRANSFER_ID);
    assert.equal(call.fn_name, "deposit");

    // Get decoded event (driver & bytes hidden by contract facade)
    const decoded = await deposit$;
    console.log('[decoded event] "deposit" ->', decoded);

    const amt = decoded?.value ?? decoded?.amount;
    if (amt !== undefined) {
      assert.equal(BigInt(amt), AMOUNT);
    }
  } finally {
    await network.disconnect();
  }
});

Deno.test("encode_ds_address encodes with driver", async () => {
  // Dummy value, the underlying contract is not being called
  const BRIDGE_ID = "0300000000000000000000000000000000000000000000000000000000000000";
  const WASM_PATH = new URL("./assets/standard_bridge_dd.wasm", import.meta.url).pathname;

  const network   = await Network.connect("https://devnet.nodes.dusk.network/");
  const wasmBytes = await Deno.readFile(WASM_PATH);
  network.dataDrivers.register(BRIDGE_ID, () => wasmBytes);

  // No bookentry needed for encode()
  const bridge = new Contract({
    contractId: BRIDGE_ID,
    network,
    driver: network.dataDrivers.get(BRIDGE_ID),
  });

  // Pass a dummy PublicKey to be encoded
  const pkArg =
    "26brdzqNXEG1jTzCubJAPhks18bSSDY4n21ZW6VLYkCv6bBUdBAZZAbn1Coz1LPBYc4uEekBbzFnZvhL9untGCqRamhZS2cBV51fdZog3qkP3NbMEaqgNMcKEahAFV8t2Cke"

  // Call encode directly, returns Uint8Array RKYV bytes
  const rkyv = await bridge.encode("encode_ds_address", pkArg);

  console.log("RKYV len:", rkyv.length);
  console.log("RKYV:", rkyv);
  if (!(rkyv instanceof Uint8Array)) {
    throw new Error("encode() did not return bytes");
  }
  if (rkyv.length === 0) {
    throw new Error("encode() returned empty payload");
  }

  await network.disconnect();
});

