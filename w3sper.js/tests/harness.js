// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../src/protocol-driver/mod.js";
import {
  test as harnessTest,
  assert,
} from "../../../../nayma/mio-mini/test-harness/mod.js";

export { assert };

export async function test(name, fn) {
  let path = "";
  switch (test.withLocalWasm) {
    case "debug":
      path = "../target/wasm32-unknown-unknown/debug/wallet_core.wasm";
      break;
    case "release":
      path = "../target/wasm32-unknown-unknown/release/wallet_core.wasm";
      break;
  }

  if (path.length > 0 && typeof Deno !== "undefined") {
    const testFn = async (...args) => {
      const wasm = await Deno.readFile(path);

      ProtocolDriver.load(
        wasm,
        new URL("./assets/debug-imports.js", import.meta.url),
      );

      await Promise.resolve(fn(...args)).finally(ProtocolDriver.unload);
    };

    return harnessTest(name, testFn);
  }
}

// Define a seed for deterministic profile generation
const SEED = new Uint8Array([
  153, 16, 102, 99, 133, 196, 55, 237, 42, 2, 163, 116, 233, 89, 10, 115, 19,
  81, 140, 31, 38, 81, 10, 46, 118, 112, 151, 244, 145, 90, 145, 168, 214, 242,
  68, 123, 116, 76, 223, 56, 200, 60, 188, 217, 34, 113, 55, 172, 27, 255, 184,
  55, 143, 233, 109, 20, 137, 34, 20, 196, 252, 117, 221, 221,
]);

export const seeder = () => SEED;

export class Treasury {
  #users;
  #notes = new Map();
  #accounts = [];

  constructor(users) {
    this.#users = users;

    users.forEach((user) => {
      this.#notes.set(user.address.toString(), new Map());
    });
  }

  async read({ from, addresses, accounts }) {
    if (accounts) {
      this.#accounts = await accounts.balances(this.#users);
    }

    if (!addresses) {
      return;
    }

    for await (let [notes] of await addresses.notes(this.#users, {
      from,
    })) {
      for (let i = 0; i < this.#users.length; i++) {
        const userNotes = this.#notes.get(this.#users[i].address.toString());
        this.#notes.set(
          this.#users[i].address.toString(),
          new Map([...userNotes, ...notes[i]]),
        );
      }
    }
  }

  address(profile) {
    return this.#notes.get(profile.toString());
  }

  account(profile) {
    return this.#accounts.at(+profile);
  }
}
