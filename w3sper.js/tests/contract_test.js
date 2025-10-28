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
    assert.ok(chainId == 0);
  } finally {
    await network.disconnect();
  }
});

test("contract.call feeder: transfer.sync_accounts", async () => {
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

    // Pass a number tuple to the transfer contract `sync_accounts` function, and
    // set `feeder: true` to stream the output.
    const result = await transfer.call.sync_accounts(["0", "5"], { feeder: true });

    // Validate balances/nonce object of genesis account
    assert.equal(result[0], { balance: "1001000000000000", nonce: "0" });
  } finally {
    await network.disconnect();
  }
});

test("contract.encode: transfer.sync_account", async () => {
  const INPUT = ["0", "5"];
  const OUTPUT = new Uint8Array([
    0, 0, 0, 0, 0, 0,
    0, 0, 5, 0, 0, 0,
    0, 0, 0, 0
  ]);

  const network = await Network.connect(NETWORK);
  try {
    // Register the genesis transfer driver
    network.dataDrivers.register(TRANSFER_ID, () => readDriverFromTarget(TRANSFER_WASM));
    const driver = network.dataDrivers.get(TRANSFER_ID);
    const transfer = new Contract({ contractId: TRANSFER_ID, driver, network });

    // Encode request body for `sync_account`
    const rkyv = await transfer.encode("sync_accounts", INPUT);

    assert.ok(rkyv instanceof Uint8Array && rkyv.length > 0, "encode() must return non-empty bytes");
    assert.equal(rkyv, OUTPUT);

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
