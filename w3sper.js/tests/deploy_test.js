// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  ContractDeployment,
  Network,
  ProfileGenerator,
  useAsProtocolDriver,
} from "@dusk/w3sper";
import * as bip39 from "bip39";

import { assert, getLocalWasmBuffer, test } from "./harness.js";

const RECOVERY_PHRASE_URL = new URL(
  "../../examples/recovery-phrase.txt",
  import.meta.url,
);
const ALICE_WASM_URL = new URL("./assets/alice.wasm", import.meta.url);

test("offline moonlight contract deployment", async () => {
  await useAsProtocolDriver(await getLocalWasmBuffer()).then(async () => {
    const phrase = (await Deno.readTextFile(RECOVERY_PHRASE_URL)).trim();
    const profiles = new ProfileGenerator(() => bip39.mnemonicToSeed(phrase));
    const profile = await profiles.default;
    const bytecode = await Deno.readFile(ALICE_WASM_URL);

    const tx = await new ContractDeployment(profile, bytecode, {
      nonce: 42n,
    })
      .accountNonce(0n)
      .chain(Network.LOCALNET)
      .gas({ limit: 120_000_000n, price: 2_000n })
      .build();

    assert.equal(tx.nonce, 1n);
    assert.equal(
      tx.contractId,
      "36e42c09f179ab0e0fcc004e0a75aa3f5dfc405cf7e15f8330d1d805d4093dcd",
    );
    assert.equal(tx.hash.length, 64);
    assert.ok(tx.buffer.byteLength > bytecode.byteLength);
  });
});
