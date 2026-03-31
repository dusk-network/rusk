// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { SyncEvent } from "./event.js";
import { getBYOBReader } from "../../protocol-driver/stream.js";
import * as DataBuffer from "../../protocol-driver/buffer.js";
import * as ProtocolDriver from "../../protocol-driver/mod.js";
import { Bookmark } from "../bookmark.js";

const DEFAULT_OWNERSHIP_CHUNK_SIZE = 512;
const MAX_OWNERSHIP_WORKERS = 16;
const DEFAULT_NULLIFIER_BATCH_BYTE_BUDGET = 60 * 1024;
const NULLIFIER_BATCH_LAYOUT_BYTES = 8;
const size = (array) => array.reduce((sum, { size }) => sum + size, 0);

function clampPositiveInteger(value, fallback) {
  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function defaultOwnershipWorkers() {
  return Math.min(
    clampPositiveInteger(globalThis.navigator?.hardwareConcurrency, 1),
    MAX_OWNERSHIP_WORKERS,
  );
}

function createDeferred() {
  let resolve, reject;
  const promise = new Promise((innerResolve, innerReject) => {
    resolve = innerResolve;
    reject = innerReject;
  });

  return { promise, resolve, reject };
}

function mergeOwnedBatch(dest, source) {
  for (let i = 0; i < source.length; i++) {
    for (const [key, value] of source[i]) {
      dest[i].set(key, value);
    }
  }
}

function splitNoteChunk(notes, itemSize, chunkSize) {
  const chunkByteLength = itemSize * chunkSize;
  const chunks = [];

  for (let offset = 0; offset < notes.byteLength; offset += chunkByteLength) {
    const end = Math.min(offset + chunkByteLength, notes.byteLength);
    chunks.push(notes.slice(offset, end));
  }

  return chunks;
}

function encodeNullifierBatch(nullifiers, itemSize) {
  const byteLength = nullifiers.length * itemSize +
    NULLIFIER_BATCH_LAYOUT_BYTES;
  const body = new Uint8Array(byteLength);

  DataBuffer.copyInto(body, nullifiers);
  DataBuffer.layout(body, nullifiers.length);

  return body;
}

function splitNullifierBatches(nullifiers, itemSize) {
  if (nullifiers.length === 0) {
    return [];
  }

  const batchSize = Math.max(
    Math.floor(
      Math.max(
        DEFAULT_NULLIFIER_BATCH_BYTE_BUDGET - NULLIFIER_BATCH_LAYOUT_BYTES,
        0,
      ) / itemSize,
    ),
    1,
  );
  const batches = [];

  for (let index = 0; index < nullifiers.length; index += batchSize) {
    batches.push(nullifiers.slice(index, index + batchSize));
  }

  return batches;
}

function assertProfilesShareSource(profiles) {
  if (profiles.length === 0) {
    return;
  }

  const firstProfile = profiles[0];
  const sharesSameSource = profiles.every((profile) =>
    firstProfile.sameSourceOf(profile)
  );

  if (!sharesSameSource) {
    throw new Error("All owners must be generated from the same source");
  }
}

function workerURL() {
  return new URL("./owned_worker.js", import.meta.url);
}

class OwnedWorker {
  #worker;
  #pending = new Map();
  #ready = createDeferred();
  #nextId = 0;

  constructor(wasmSource, seed, indexes) {
    this.#worker = new Worker(workerURL(), { type: "module" });
    this.#worker.onmessage = ({ data }) => {
      if (data?.type === "ready") {
        this.#ready.resolve();
        return;
      }

      if (data?.type === "error") {
        const pending = this.#pending.get(data.id);
        if (pending) {
          this.#pending.delete(data.id);
          pending.reject(new Error(data.message));
        } else {
          this.#ready.reject(new Error(data.message));
        }
        return;
      }

      if (data?.type === "result") {
        const pending = this.#pending.get(data.id);
        if (!pending) {
          return;
        }

        this.#pending.delete(data.id);
        pending.resolve([data.owned, data.syncInfo]);
      }
    };

    this.#worker.onerror = (event) => {
      const error = event.error ?? new Error(event.message);
      for (const pending of this.#pending.values()) {
        pending.reject(error);
      }
      this.#pending.clear();
      this.#ready.reject(error);
    };

    this.#worker.postMessage({
      type: "init",
      wasmSource,
      seed,
      indexes,
    });
  }

  async mapOwned(notes) {
    await this.#ready.promise;

    const id = this.#nextId++;
    const deferred = createDeferred();
    this.#pending.set(id, deferred);
    this.#worker.postMessage({ type: "map", id, notes }, [notes.buffer]);

    return deferred.promise;
  }

  close() {
    const error = new Error("Owned note worker terminated");
    for (const pending of this.#pending.values()) {
      pending.reject(error);
    }
    this.#pending.clear();
    this.#worker.terminate();
  }
}

class OwnedWorkerPool {
  #workers;
  #chunkSize;
  #itemSize;

  constructor({ size, wasmSource, seed, indexes, itemSize, chunkSize }) {
    this.#workers = Array.from(
      { length: size },
      () => new OwnedWorker(wasmSource, seed, indexes),
    );
    this.#itemSize = itemSize;
    this.#chunkSize = chunkSize;
  }

  async mapOwned(notes) {
    const noteChunks = splitNoteChunk(notes, this.#itemSize, this.#chunkSize);
    const mappedChunks = await Promise.all(
      noteChunks.map((chunk, index) => this.#workers[index].mapOwned(chunk)),
    );

    const owned = mappedChunks[0][0].map(() => new Map());
    let blockHeight = 0n;
    let bookmark = 0n;

    for (const [chunkOwned, syncInfo] of mappedChunks) {
      mergeOwnedBatch(owned, chunkOwned);
      if (syncInfo.blockHeight > blockHeight) {
        blockHeight = syncInfo.blockHeight;
      }
      if (syncInfo.bookmark > bookmark) {
        bookmark = syncInfo.bookmark;
      }
    }

    return [owned, { blockHeight, bookmark }];
  }

  close() {
    this.#workers.forEach((worker) => worker.close());
  }
}

/**
 * Synchronizes Phoenix notes and nullifier state for one or more profiles.
 */
export class AddressSyncer extends EventTarget {
  #network;
  #ownershipWorkers;
  #ownershipChunkSize;

  constructor(network, options = {}) {
    super();
    this.#network = network;
    this.#ownershipWorkers = clampPositiveInteger(
      options.ownershipWorkers,
      defaultOwnershipWorkers(),
    );
    this.#ownershipChunkSize = clampPositiveInteger(
      options.ownershipChunkSize,
      DEFAULT_OWNERSHIP_CHUNK_SIZE,
    );
  }

  get #bookmark() {
    return this.#network.contracts.transferContract.call
      .num_notes()
      .then((response) => response.arrayBuffer())
      .then((buffer) => new Bookmark(new Uint8Array(buffer)));
  }

  get root() {
    const network = this.#network;
    return network.contracts.transferContract.call
      .root()
      .then((response) => response.arrayBuffer());
  }

  async openings(notes, _options = {}) {
    const network = this.#network;
    // Get the bookmarks for each notes
    const bookmarks = await ProtocolDriver.bookmarks(notes);

    // Fetch the openings for each picked notes
    const requests = [];

    for (let i = 0; i < bookmarks.length; i += 8) {
      const body = bookmarks.slice(i, i + 8);

      requests.push(network.contracts.transferContract.call.opening(body));
    }

    const result = Promise.all(requests)
      .then((responses) => responses.map((response) => response.arrayBuffer()))
      .then((buffers) => Promise.all(buffers));
    return result;
  }

  async spent(nullifiers) {
    const entrySize = await ProtocolDriver.getEntrySize();
    const spentNullifiers = [];

    for (const batch of splitNullifierBatches(nullifiers, entrySize.key)) {
      const body = encodeNullifierBatch(batch, entrySize.key);
      const response = await this.#network.contracts.transferContract.call
        .existing_nullifiers(body);
      if (!response.ok) {
        throw new Error(await response.text());
      }

      const buffer = await response.arrayBuffer();
      const layout = DataBuffer.layout(new Uint8Array(buffer));

      for (let i = 0; i < layout.totalLength; i += entrySize.key) {
        spentNullifiers.push(buffer.slice(i, i + entrySize.key));
      }
    }

    return spentNullifiers;
  }

  async notes(profiles, options = {}) {
    const from = options.from ?? 0n;
    const lastBookmark = await this.#bookmark;
    const lastBlock = await this.#network.blockHeight;

    let body, topic;

    if (from instanceof Bookmark) {
      topic = "leaves_from_pos";
      body = from.data;
    } else {
      topic = "leaves_from_height";
      body = new Uint8Array(8);
      new DataView(body.buffer).setBigUint64(0, from, true);
    }

    assertProfilesShareSource(profiles);

    const entrySize = await ProtocolDriver.getEntrySize();
    const workerCount = clampPositiveInteger(
      this.#ownershipWorkers,
      defaultOwnershipWorkers(),
    );
    const canUseWorkers = workerCount > 1 && typeof Worker === "function" &&
      profiles.length > 0;
    const workerThresholdBytes = entrySize.item * this.#ownershipChunkSize;
    const wasmSource = new URL(
      "/static/drivers/wallet-core-1.6.0.wasm",
      this.#network.url,
    ).toString();
    const indexes = profiles.map((profile) => +profile);
    let workerPool = null;

    const response = await this.#network.contracts.transferContract.call[topic](
      body,
      { headers: { "Rusk-Feeder": "true" } },
    );
    const reader = getBYOBReader(response.body);

    const stream = new ReadableStream({
      pull: async (controller) => {
        try {
          const batchNotes = this.#ownershipChunkSize *
            Math.max(workerCount, 1);
          const buffer = new Uint8Array(entrySize.item * batchNotes);

          const { done, value } = await reader.read(buffer);

          if (done) {
            workerPool?.close();
            controller.close();
            return;
          }

          if (
            canUseWorkers &&
            !workerPool &&
            value.byteLength > workerThresholdBytes
          ) {
            workerPool = new OwnedWorkerPool({
              size: workerCount,
              wasmSource,
              seed: new Uint8Array(await profiles[0].seed),
              indexes,
              itemSize: entrySize.item,
              chunkSize: this.#ownershipChunkSize,
            });
          }

          const [owned, syncInfo] = workerPool
            ? await workerPool.mapOwned(value)
            : await ProtocolDriver.mapOwned(profiles, value);

          const progress = Number(
            ((syncInfo.bookmark * 100n) / lastBookmark.asUint()) * 100n,
          ) / 10000;

          this.dispatchEvent(
            new SyncEvent("synciteration", {
              ownedCount: size(owned),
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
          workerPool?.close();
          await reader.cancel(error);
          controller.error(error);
        }
      },
      cancel(reason) {
        workerPool?.close();
        console.log("Stream canceled:", reason);
      },
    });

    return stream;
  }
}
