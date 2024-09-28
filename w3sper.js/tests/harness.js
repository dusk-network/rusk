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
      path = `../target/wasm32-unknown-unknown/debug/wallet_core.wasm`;
      break;
    case "release":
      path = `../target/wasm32-unknown-unknown/release/wallet_core.wasm`;
      break;
  }

  if (path.length > 0 && typeof Deno !== "undefined") {
    const wasm = await Deno.readFile(path);

    const testFn = async (...args) => {
      ProtocolDriver.load(
        wasm,
        new URL("./assets/debug-imports.js", import.meta.url),
      );

      await Promise.resolve(fn(...args)).finally(ProtocolDriver.unload);
    };

    return harnessTest(name, testFn);
  }
}
