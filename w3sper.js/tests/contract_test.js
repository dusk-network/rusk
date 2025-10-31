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
    "", // ADD MNEMONIC
  );

// Wrapper function to grab a data-driver from the Rust release folder
async function readDriverFromTarget(name) {
  const url = new URL(
    `./${name}`,
    import.meta.url,
  );
  return new Uint8Array(await Deno.readFile(url));
}

const BRIDGE_CONTRACT_ID = "244d941cd4fbdbcb97c2b0083c6d528cf5f02ddbe614fce08884aad119a339c4";

test("bridge pending withdrawals", async () => {
  const network = await Network.connect("https://devnet.nodes.dusk.network/");
  try {
    network.dataDrivers.register(
      BRIDGE_CONTRACT_ID,
      () => readDriverFromTarget("standard_bridge_dd_opt.wasm"),
    );
    const bridgeContract = new Contract({
      contractId: BRIDGE_CONTRACT_ID,
      driver: network.dataDrivers.get(BRIDGE_CONTRACT_ID),
      network,
    });

    const pendingWithdrawals = await bridgeContract.call.pending_withdrawals(undefined, { feeder: true });

    console.log(pendingWithdrawals);
  } finally {
    await network.disconnect();
  }
});

test("bridge add deposit", async () => {
  const network = await Network.connect("https://devnet.nodes.dusk.network/");
  try {
    const profiles = new ProfileGenerator(seeder);
    const users = [await profiles.default];
    const accounts = new AccountSyncer(network);
    const treasury = new Treasury(users);
    await treasury.update({ accounts });

    network.dataDrivers.register(
      BRIDGE_CONTRACT_ID,
      () => readDriverFromTarget("standard_bridge_dd_opt.wasm"),
    );
    const bookentry = new Bookkeeper(treasury).as(users[0]);
    const bridgeContract = bookentry.contract(BRIDGE_CONTRACT_ID, network);

    // Execute with matching args <-> deposit
    const AMOUNT_TO_BRIDGE = 2_000_000_000;
    const BRIDGE_FEE = 500_000;
    const TOTAL_AMOUNT = BigInt(2_000_500_000);
    const GAS_LIMIT = BigInt(1_000_000)

    const payload = {
      to: "0x8943545177806ed17b9f23f0a21ee5948ecaa776",
      amount: AMOUNT_TO_BRIDGE,
      fee: BRIDGE_FEE,
      extra_data: ""
    }

    const builder = await bridgeContract.tx.deposit(payload);
    const { hash } = await network.execute(
      builder.to(users[0].account).deposit(TOTAL_AMOUNT).gas({ limit: GAS_LIMIT }),
    );
    assert.ok(typeof hash === "string" && hash.length > 0);

    // Check that the transaction is indeed executed
    const executed = await network.transactions.withId(hash).once.executed();
    const call = executed.call();
    assert.equal(call.contract, BRIDGE_CONTRACT_ID);
    assert.equal(call.fn_name, "deposit");
  } finally {
    await network.disconnect();
  }
});
