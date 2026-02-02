// we are importing the file directly to avoid importing our own mock
import { AddressSyncer } from "$lib/../../node_modules/@dusk/w3sper/src/network/syncer/address";

import { cacheUnspentNotes } from "$lib/mock-data";

import notesArrayToMap from "$lib/wallet/notesArrayToMap";

/**
 * @template T
 * @param {Array<T>} arr
 * @param {number} n
 */
function* chunkGenerator(arr, n) {
  const chunkSize = Math.floor(arr.length / n);

  let start = 0;

  for (let i = 0; i < n; i++) {
    const end = start + chunkSize + (i === n - 1 ? arr.length % n : 0);
    yield arr.slice(start, end);
    start = end;
  }
}

class SyncEvent extends CustomEvent {
  /**
   * @param {string} type
   * @param {Record<string, any>} detail
   */
  constructor(type, detail) {
    super(type, { detail });
  }
}

class AddressSyncerMock extends AddressSyncer {
  /**
   * @param {Array<Profile>} profiles
   * @param {Record<string, any>} [options={}]
   * @returns {Promise<ReadableStream<[Array<Map<Uint8Array, Uint8Array>>, { blockHeight: bigint; bookmark: bigint; }]>>}
   */
  // eslint-disable-next-line no-unused-vars
  async notes(profiles, options = {}) {
    const addresses = profiles.map((profile) => profile.address.toString());
    const notesAsMap = notesArrayToMap(cacheUnspentNotes);
    const notesArray = addresses.map(
      (address) => notesAsMap.get(address) ?? new Map()
    );

    let currentChunk = 0;

    const generator = chunkGenerator(notesArray, 4);

    return new ReadableStream({
      pull: async (controller) => {
        const { done, value } = generator.next();

        if (done) {
          controller.close();

          return;
        }

        /** @type {{ blockHeight: bigint, bookmark: bigint }} */
        const syncInfo = {
          blockHeight: 50n * BigInt(currentChunk),
          bookmark: 100n * BigInt(currentChunk),
        };

        this.dispatchEvent(
          new SyncEvent("synciteration", {
            blocks: {
              current: syncInfo.blockHeight,
              last: 150n,
            },
            bookmarks: {
              current: syncInfo.bookmark,
              last: 300n,
            },
            ownedCount: notesArray.length,
            progress: 0.25 * (currentChunk + 1),
          })
        );

        controller.enqueue([value, syncInfo]);
        currentChunk++;
      },
    });
  }

  /**
   * @param {Uint8Array[]} nullifiers
   * @returns {Promise<ArrayBuffer[]>}
   */
  // eslint-disable-next-line no-unused-vars
  async spent(nullifiers) {
    return [];
  }
}

export default AddressSyncerMock;
