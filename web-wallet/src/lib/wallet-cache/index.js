import { Dexie } from "dexie";
import { getKey, head, pluckFrom, uniquesBy } from "lamb";

const uniquesById = uniquesBy(getKey("id"));

class WalletCache {
  /** @type {Dexie} */
  #db;

  /**
   * @template {"history" | "spentNotes" | "unspentNotes"} TName
   * @param {TName} tableName
   * @param {string} [psk]
   * @returns {Promise<TName extends "history" ? WalletCacheHistoryEntry[] : WalletCacheNote[]>}
   */
  async #getEntriesFrom(tableName, psk) {
    await this.#db.open();

    const table = this.#db.table(tableName);
    const data =
      /** @type {import("dexie").PromiseExtended<TName extends "history" ? WalletCacheHistoryEntry[] : WalletCacheNote[]>} */ (
        (psk ? table.where("psk").equals(psk) : table).toArray()
      );

    return data.finally(() => this.#db.close());
  }

  constructor() {
    const db = new Dexie("@dusk-network/wallet-cache");

    db.version(1).stores({
      history: "psk",
      spentNotes: "nullifier,&pos,psk",
      unspentNotes: "nullifier,&pos,psk",
    });

    this.#db = db;
  }

  /**
   * @param {WalletCacheNote[]} notes
   * @returns {Promise<void>}
   */
  async addSpentNotes(notes) {
    const keysToRemove = pluckFrom(notes, "nullifier");

    await this.#db.open();

    return this.#db
      .transaction("rw", ["spentNotes", "unspentNotes"], async () => {
        await Promise.all([
          this.#db.table("spentNotes").bulkPut(notes),
          this.#db.table("unspentNotes").bulkDelete(keysToRemove),
        ]);
      })
      .finally(() => this.#db.close());
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

  /** @returns {Promise<void>} */
  clear() {
    return this.#db.delete({ disableAutoOpen: false });
  }

  /**
   * @param {string} [psk] bs58 encoded public spend key to fetch the notes of
   * @returns {Promise<WalletCacheNote[]>}
   */
  getAllNotes(psk) {
    return Promise.all([
      this.getUnspentNotes(psk),
      this.getSpentNotes(psk),
    ]).then(([unspent, spent]) => unspent.concat(spent));
  }

  /**
   * @param {string} psk bs58 encoded public spend key to fetch the tx history of
   * @return {Promise<WalletCacheHistoryEntry | undefined>}
   */
  async getHistoryEntry(psk) {
    return this.#getEntriesFrom("history", psk).then(head);
  }

  /**
   * @param {string} [psk] bs58 encoded public spend key to fetch the spent notes of
   * @returns {Promise<WalletCacheNote[]>}
   */
  getSpentNotes(psk) {
    return this.#getEntriesFrom("spentNotes", psk);
  }

  /**
   * @param {string} [psk] bs58 encoded public spend key to fetch the unspent notes of
   * @returns {Promise<WalletCacheNote[]>}
   */
  getUnspentNotes(psk) {
    return this.#getEntriesFrom("unspentNotes", psk);
  }

  /**
   * @param {WalletCacheHistoryEntry} entry
   * @returns {Promise<void>}
   */
  async setHistoryEntry(entry) {
    await this.#db.open();

    return this.#db
      .transaction("rw", "history", async () => {
        const { psk } = entry;

        /**
         * Typescript here doesn't get the `undefined` case
         * so we set the type explicitely.
         *
         * @type {WalletCacheHistoryEntry | undefined}
         */
        const current = await this.#getEntriesFrom("history", psk).then(head);

        await this.#db.table("history").put({
          history: uniquesById(
            current ? entry.history.concat(current.history) : entry.history
          ),
          lastBlockHeight: current
            ? Math.max(current.lastBlockHeight, entry.lastBlockHeight)
            : entry.lastBlockHeight,
          psk,
        });
      })
      .finally(() => this.#db.close());
  }
}

export default new WalletCache();
