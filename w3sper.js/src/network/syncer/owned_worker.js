// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../../protocol-driver/mod.js";

let workerSeed = null;
let workerIndexes = [];

function buildOwners() {
  return workerIndexes.map((index) => ({
    sameSourceOf() {
      return true;
    },
    get seed() {
      return Promise.resolve(workerSeed);
    },
    [Symbol.toPrimitive](hint) {
      if (hint === "number") {
        return index;
      }

      return null;
    },
  }));
}

globalThis.onmessage = async ({ data }) => {
  try {
    if (data?.type === "init") {
      ProtocolDriver.load(data.wasmSource);
      await ProtocolDriver.getEntrySize();
      workerSeed = new Uint8Array(data.seed);
      workerIndexes = Array.from(data.indexes);
      globalThis.postMessage({ type: "ready" });
      return;
    }

    if (data?.type === "map") {
      const [owned, syncInfo] = await ProtocolDriver.mapOwned(
        buildOwners(),
        data.notes,
      );

      globalThis.postMessage({
        type: "result",
        id: data.id,
        owned,
        syncInfo,
      });
    }
  } catch (error) {
    globalThis.postMessage({
      type: "error",
      id: data?.id,
      message: error?.message ?? String(error),
    });
  }
};
