// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../protocol-driver.js";
import { getBYOBReader } from "../protocol-driver/stream.js";
import { Bookmark } from "./state-syncer/bookmark.js";

export const TRANSFER =
  "0100000000000000000000000000000000000000000000000000000000000000";

export class SyncEvent extends CustomEvent {
  constructor(type, detail) {
    super(type, { detail });
  }
}

export class StateSyncer extends EventTarget {
  #network;

  constructor(network, options = {}) {
    const { signal } = options;

    super();
    this.#network = network;
  }

  get #bookmark() {
    const url = new URL(
      `/on/contracts:${TRANSFER}/num_notes`,
      this.#network.url,
    );

    const request = new Request(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/octet-stream",
      },
    });

    return this.#network
      .dispatch(request)
      .then((response) => response.arrayBuffer())
      .then((buffer) => new Bookmark(new Uint8Array(buffer)));
  }

  async accounts(users) {
    // await ProtocolDriver.accountsIntoRaw(users);
    // const url = new URL(
    //   `/on/contracts:${TRANSFER}/account`,
    //   this.#network.url,
    //   pub fn account(&self, key: &AccountPublicKey) -> AccountData {
    //       let key_bytes = key.to_raw_bytes();
    //       self.accounts
    //           .get(&key_bytes)
    //           .cloned()
    //           .unwrap_or(EMPTY_ACCOUNT)
    //   }
    // );
  }

  async notes(owners, options = {}) {
    const from = options.from ?? 0n;
    const lastBookmark = await this.#bookmark;
    const lastBlock = await this.#network.blockHeight;

    let body, topic, to;

    if (from instanceof Bookmark) {
      topic = "leaves_from_pos";
      body = from.data;
      to = lastBookmark;
    } else {
      topic = "leaves_from_height";
      body = new Uint8Array(8);
      to = lastBlock;
      new DataView(body.buffer).setBigUint64(0, from, true);
    }

    const url = new URL(
      `/on/contracts:${TRANSFER}/${topic}`,
      this.#network.url,
    );

    const request = new Request(url, {
      method: "POST",
      headers: {
        "Rusk-Feeder": "true",
        "Content-Type": "application/octet-stream",
      },
      body,
    });

    let response = await this.#network.dispatch(request);
    let reader = getBYOBReader(response.body);

    const entrySize = await ProtocolDriver.getEntrySize();

    const stream = new ReadableStream({
      pull: async (controller) => {
        try {
          const buffer = new Uint8Array(entrySize.item * 100); // Pre-allocated buffer

          const { done, value } = await reader.read(buffer);

          if (done) {
            controller.close();
            return;
          }

          const [owned, syncInfo] = await ProtocolDriver.mapOwned(
            owners,
            value,
          ).catch(console.error);

          let progress =
            Number(
              ((syncInfo.bookmark * 100n) / lastBookmark.asUint()) * 100n,
            ) / 10000;

          this.dispatchEvent(
            new SyncEvent("synciteration", {
              ownedCount: owned.size,
              progress,
              bookmarks: {
                current: syncInfo.bookmark,
                last: lastBookmark.asUint(),
              },
              blocks: {
                current: syncInfo.blockHeight,
                last: lastBlock,
              },
            }),
          );

          // Enqueue the result [owned, syncInfo] into the stream
          controller.enqueue([owned, syncInfo]);
        } catch (error) {
          console.error("Error processing stream:", error);
          controller.error(error); // Signal an error in the stream
        }
      },
      cancel(reason) {
        console.log("Stream canceled:", reason);
      },
    });

    return stream;
  }
}
