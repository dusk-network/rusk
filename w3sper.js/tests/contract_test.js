// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  Contract,
  Network,
} from "../src/mod.js";
import { test } from "./harness.js";
import * as bip39 from "npm:bip39";

// Generate 64 byte seed from the mnemonic.
export const seeder = async () =>
  await bip39.mnemonicToSeed(
    "eyebrow milk fan usage maximum exhibit ski nut wing sell alone great",
  );

// Wrapper function to grab a data-driver from the Rust release folder
async function readDriverFromTarget(name) {
  const url = new URL(
    `./${name}`,
    import.meta.url,
  );
  return new Uint8Array(await Deno.readFile(url));
}

test("bridge pending withdrawals", async () => {
  const network = await Network.connect("https://devnet.nodes.dusk.network/");
  try {
    network.dataDrivers.register(
      "244d941cd4fbdbcb97c2b0083c6d528cf5f02ddbe614fce08884aad119a339c4",
      () => readDriverFromTarget("standard_bridge_dd_opt.wasm"),
    );
    const transfer = new Contract({
      contractId: "244d941cd4fbdbcb97c2b0083c6d528cf5f02ddbe614fce08884aad119a339c4",
      driver: network.dataDrivers.get("244d941cd4fbdbcb97c2b0083c6d528cf5f02ddbe614fce08884aad119a339c4"),
      network,
    });

    const pendingWithdrawals = await transfer.call.pending_withdrawals(undefined, { feeder: true });

    console.log(pendingWithdrawals);
  } finally {
    await network.disconnect();
  }
});
