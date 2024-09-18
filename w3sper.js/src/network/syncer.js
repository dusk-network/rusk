// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../protocol-driver.js";
import { Bookmark } from "./syncer/bookmark.js";
import { getBYOBReader } from "../stream.js";

export const TRANSFER =
  "0100000000000000000000000000000000000000000000000000000000000000";

function prepare(chunks, size) {
  let totalLength = chunks.length;

  // Allocate a Uint8Array with the total length,
  // plus 4 bytes for the buffer size and 8 bytes
  // for the struct layout
  const result = new Uint8Array(totalLength + 8);
  const view = new DataView(result.buffer);

  result.set(chunks);

  // Calculate and copy the layout
  const align = 2 ** 32 - ((totalLength + 3) & ~3);

  view.setUint32(result.length - 8, align, true);
  view.setUint32(result.length - 4, size, true);

  return result;
}

export class SyncEvent extends CustomEvent {
  constructor(type, detail) {
    super(type, { detail });
  }
}

export class Syncer extends EventTarget {
  #network;
  #from;

  constructor(network, options = {}) {
    const { from, signal } = options;

    super();
    this.#network = network;
    this.#from = from ?? 0n;
  }

  get from() {
    return this.#from;
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

  async entriesFor(owners) {
    const from = this.#from;
    const lastBookmark = await this.#bookmark;
    const lastBlock = await this.#network.blockHeight;

    let body, topic, to;

    if (from instanceof Bookmark) {
      topic = "leaves_from_pos";
      body = this.#from.data;
      to = lastBookmark;
    } else {
      topic = "leaves_from_height";
      body = new Uint8Array(8);
      to = lastBlock;
      new DataView(body.buffer).setBigUint64(0, this.#from, true);
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
            prepare(value, value.byteLength / entrySize.item),
          ).catch(console.error);

          let progress =
            Number(
              ((syncInfo.bookmark * 100n) / lastBookmark.asUintN()) * 100n,
            ) / 10000;

          this.dispatchEvent(
            new SyncEvent("synciteration", {
              ownedCount: owned.size,
              progress,
              bookmarks: {
                current: syncInfo.bookmark,
                last: lastBookmark.asUintN(),
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
