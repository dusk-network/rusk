import { AddressSyncer } from "$lib/vendor/w3sper.js/src/network/syncer/address";

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
   * @param {import("$lib/vendor/w3sper.js/src/mod").Network} network
   * @param {Record<string, any>} [options={}]
   */
  constructor(network, options = {}) {
    super(network, options);
  }

  /**
   * @param {Array<import("$lib/vendor/w3sper.js/src/mod").Profile>} profiles
   * @param {Record<string, any>} [options={}]
   * @returns {Promise<ReadableStream<any>>}
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

        /** @type {WalletCacheSyncInfo} */
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
   * @returns {Promise<Uint8Array[]>}
   */
  // eslint-disable-next-line no-unused-vars
  async spent(nullifiers) {
    return [];
  }
}

export default AddressSyncerMock;
