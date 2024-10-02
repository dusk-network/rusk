import { Dexie } from "dexie";
import { head, isUndefined, pairs, pipe, skipIf, unless, when } from "lamb";

/**
 * Not importing from "$lib/wallet" because for some reason
 * while running tests the export of `initializeWallet` there
 * causes this bug to happen:
 * https://github.com/nodejs/undici/issues/2663
 */
import notesArrayToMap from "$lib/wallet/notesArrayToMap";

/** @typedef {{ nullifiers?: Uint8Array[] } | { addresses?: string[] }} RawCriteria */
/** @typedef {{ field: "nullifier", values: Uint8Array[] } | { field: "address", values: string[]} | undefined} Criteria */

/** @type {(rawCriteria: RawCriteria) => Criteria} */
const toCriteria = pipe([
  skipIf(isUndefined),
  pairs,
  head,
  unless(isUndefined, (pair) => ({
    field: pair[0] === "nullifiers" ? "nullifier" : "address",
    values: pair[1],
  })),
]);

class WalletCache {
  /** @type {Dexie} */
  #db;

  /** @type {WalletCacheTreasury} */
  #treasury = {
    address: async (profile) => {
      /** @type {WalletCacheNote[]} */
      const result = [];
      const address = profile.address.toString();
      const notes = await this.getUnspentNotes([address]);

      for (const note of notes) {
        if ((await this.getPendingNotesInfo([note.nullifier])).length === 0) {
          result.push(note);
        }
      }

      return notesArrayToMap(result);
    },
  };

  /**
   * @template {WalletCacheTableName} TName
   * @template {boolean} PK
   * @param {TName} tableName
   * @param {PK} primaryKeysOnly
   * @param {RawCriteria} [rawCriteria]
   * @returns {Promise<WalletCacheGetEntriesReturnType<TName, PK>>}
   */
  async #getEntriesFrom(tableName, primaryKeysOnly, rawCriteria) {
    await this.#db.open();

    const criteria = rawCriteria && toCriteria(rawCriteria);
    const table = this.#db.table(tableName);
    const data =
      /** @type {import("dexie").PromiseExtended<WalletCacheGetEntriesReturnType<TName, PK>>} */ (
        criteria && criteria.values.length
          ? table
              .where(criteria.field)
              .anyOf(criteria.values)
              [primaryKeysOnly ? "primaryKeys" : "toArray"]()
          : primaryKeysOnly
            ? table.toCollection().primaryKeys()
            : table.toArray()
      );

    return data.finally(() => this.#db.close());
  }

  constructor() {
    const db = new Dexie("@dusk-network/wallet-cache");

    db.version(1).stores({
      pendingNotesInfo: "nullifier,txId",
      spentNotes: "nullifier,address",
      syncInfo: "++",
      unspentNotes: "nullifier,address",
    });

    this.#db = db;
  }

  /**
   * @param {WalletCacheNote[]} notes
   * @returns {Promise<void>}
   */
  async addUnspentNotes(notes) {
    await this.#db.open();

    return this.#db
      .transaction("rw", "unspentNotes", async () => {
        await this.#db.table("unspentNotes").bulkPut(notes);
      })
      .finally(() => this.#db.close());
  }

  /** @type {WalletCacheTreasury} */
  get treasury() {
    return this.#treasury;
  }

  /** @returns {Promise<void>} */
  clear() {
    return this.#db.delete({ disableAutoOpen: false });
  }

  /**
   * @param {Uint8Array[]} [nullifiers]
   * @returns {Promise<WalletCachePendingNoteInfo[]>}
   */
  getPendingNotesInfo(nullifiers) {
    return this.#getEntriesFrom("pendingNotesInfo", false, { nullifiers });
  }

  /**
   * @param {string[]} [addresses] Base58 encoded addresses to fetch the spent notes of
   * @returns {Promise<WalletCacheNote[]>}
   */
  getSpentNotes(addresses) {
    return this.#getEntriesFrom("spentNotes", false, { addresses });
  }

  /**
   * @param {string[]} [addresses] Base58 encoded addresses to fetch the spent notes of
   * @returns {Promise<Uint8Array[]>}
   */
  getSpentNotesNullifiers(addresses) {
    return this.#getEntriesFrom("spentNotes", true, { addresses });
  }

  /** @returns {Promise<WalletCacheSyncInfo>} */
  getSyncInfo() {
    return this.#getEntriesFrom("syncInfo", false)
      .then(head)
      .then(when(isUndefined, () => ({ blockHeight: 0n, bookmark: 0n })));
  }

  /**
   * @param {string[]} [addresses] Base58 encoded addresses to fetch the unspent notes of
   * @returns {Promise<WalletCacheNote[]>}
   */
  getUnspentNotes(addresses) {
    return this.#getEntriesFrom("unspentNotes", false, { addresses });
  }

  /**
   * @param {string[]} [addresses] Base58 encoded addresses to fetch the unspent notes of
   * @returns {Promise<Uint8Array[]>}
   */
  getUnspentNotesNullifiers(addresses) {
    return this.#getEntriesFrom("unspentNotes", true, { addresses });
  }
}

export default new WalletCache();
