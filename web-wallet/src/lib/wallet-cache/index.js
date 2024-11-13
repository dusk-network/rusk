import { Dexie } from "dexie";
import {
  compose,
  condition,
  getKey,
  getPath,
  head,
  isUndefined,
  mapWith,
  pairs,
  pipe,
  setKey,
  skipIf,
  unless,
  updateKey,
  when,
} from "lamb";

/** @typedef {{ nullifiers?: Uint8Array[] } | { addresses?: string[] } | { accounts?: string[] }} RawCriteria */
/** @typedef {{ field: "nullifier", values: Uint8Array[] } | { field: "address", values: string[]} | { field: "account", values: string[]} | undefined} Criteria */

/** @type {(buffer: ArrayBuffer) => Uint8Array} */
const bufferToUint8Array = (buffer) => new Uint8Array(buffer);

/** @type {(profiles: Array<import("$lib/vendor/w3sper.js/src/mod").Profile>) => string[]} */
const getAddressesFrom = mapWith(compose(String, getKey("address")));

const nullifiersToString = mapWith(String);

/** @type {(source: WalletCacheDbNote) => Omit<WalletCacheDbNote, "note"> & { note: Uint8Array }} */
const updateNote = updateKey("note", bufferToUint8Array);

const updateNullifier = updateKey("nullifier", bufferToUint8Array);

/** @type {(v: WalletCacheDbPendingNoteInfo[]) => WalletCachePendingNoteInfo[]} */
const restorePendingInfo = mapWith(updateNullifier);

/** @type {(v: WalletCacheDbNote[]) => WalletCacheNote[]} */
const restoreNotes = mapWith(compose(updateNullifier, updateNote));

const restoreNullifiers = mapWith(bufferToUint8Array);

/** @type {(rawCriteria: RawCriteria) => Criteria} */
const toCriteria = pipe([
  skipIf(isUndefined),
  pairs,
  head,
  unless(isUndefined, (pair) => ({
    field:
      pair[0] === "nullifiers"
        ? "nullifier"
        : pair[0] === "addresses"
          ? "address"
          : "account",
    values: pair[1],
  })),
]);

class WalletCache {
  /** @type {Dexie} */
  #db;

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
      balancesInfo: "address",
      pendingNotesInfo: "nullifier,txId",
      spentNotes: "nullifier,address",
      syncInfo: "++",
      unspentNotes: "nullifier,address",
    });

    db.version(2).stores({
      balancesInfo: "address",
      pendingNotesInfo: "nullifier,txId",
      spentNotes: "nullifier,address",
      stakeInfo: "account",
      syncInfo: "++",
      unspentNotes: "nullifier,address",
    });

    this.#db = db;
  }

  /**
   * @param {WalletCacheNote[]} notes
   * @param {WalletCacheSyncInfo} syncInfo
   * @returns {Promise<void>}
   */
  async addUnspentNotes(notes, syncInfo) {
    await this.#db.open();

    return this.#db
      .transaction("rw", ["syncInfo", "unspentNotes"], async () => {
        const syncInfoTable = this.#db.table("syncInfo");

        await syncInfoTable.clear();
        await syncInfoTable.add(syncInfo);
        await this.#db.table("unspentNotes").bulkPut(notes);
      })
      .finally(() => this.#db.close());
  }

  /** @returns {Promise<void>} */
  clear() {
    return this.#db.delete({ disableAutoOpen: false });
  }

  /**
   * @param {string} address
   * @returns {Promise<WalletCacheBalanceInfo["balance"]>}
   */
  getBalanceInfo(address) {
    return this.#getEntriesFrom("balancesInfo", false, {
      addresses: [address],
    })
      .then(getPath("0.balance"))
      .then(
        when(isUndefined, () => ({
          shielded: { spendable: 0n, value: 0n },
          unshielded: { nonce: 0n, value: 0n },
        }))
      );
  }

  /**
   * @param {Uint8Array[]} [nullifiers]
   * @returns {Promise<WalletCachePendingNoteInfo[]>}
   */
  getPendingNotesInfo(nullifiers) {
    return this.#getEntriesFrom("pendingNotesInfo", false, { nullifiers }).then(
      restorePendingInfo
    );
  }

  /**
   * @param {string[]} [addresses] Base58 encoded addresses to fetch the spent notes of
   * @returns {Promise<WalletCacheNote[]>}
   */
  getSpentNotes(addresses) {
    return this.#getEntriesFrom("spentNotes", false, { addresses }).then(
      restoreNotes
    );
  }

  /**
   * @param {string[]} [addresses] Base58 encoded addresses to fetch the spent notes of
   * @returns {Promise<Uint8Array[]>}
   */
  getSpentNotesNullifiers(addresses) {
    return this.#getEntriesFrom("spentNotes", true, { addresses }).then(
      restoreNullifiers
    );
  }

  /**
   * @param {string} account
   * @returns {Promise<StakeInfo>}
   */
  getStakeInfo(account) {
    return this.#getEntriesFrom("stakeInfo", false, {
      accounts: [account],
    })
      .then(getPath("0.stakeInfo"))
      .then(
        condition(
          isUndefined,
          () => ({
            amount: null,
            faults: 0,
            hardFaults: 0,
            nonce: 0n,
            reward: 0n,
          }),

          // we reinstate the `total` getter if the
          // amount is not `null`
          (stakeInfo) => ({
            ...stakeInfo,
            amount: stakeInfo.amount
              ? {
                  ...stakeInfo.amount,
                  get total() {
                    return this.value + this.locked;
                  },
                }
              : null,
          })
        )
      );
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
    return this.#getEntriesFrom("unspentNotes", false, { addresses }).then(
      restoreNotes
    );
  }

  /**
   * @param {string[]} [addresses] Base58 encoded addresses to fetch the unspent notes of
   * @returns {Promise<Uint8Array[]>}
   */
  getUnspentNotesNullifiers(addresses) {
    return this.#getEntriesFrom("unspentNotes", true, { addresses }).then(
      restoreNullifiers
    );
  }

  /**
   * Returns the array of unique nullifiers contained only
   * in the first of the two given nullifiers arrays.
   *
   * @see {@link https://en.wikipedia.org/wiki/Complement_(set_theory)#Relative_complement}
   *
   * @param {Uint8Array[]} a
   * @param {Uint8Array[]} b
   * @returns {Uint8Array[]}
   */
  nullifiersDifference(a, b) {
    if (a.length === 0 || b.length === 0) {
      return a;
    }

    const result = [];
    const lookup = new Set(nullifiersToString(b));

    for (const entry of a) {
      if (!lookup.has(entry.toString())) {
        result.push(entry);
      }
    }

    return result;
  }

  /**
   * @param {string} address
   * @param {WalletCacheBalanceInfo["balance"]} balance
   * @returns {Promise<void>}
   */
  async setBalanceInfo(address, balance) {
    return this.#db
      .open()
      .then(async (db) => {
        await db.table("balancesInfo").put({ address, balance });
      })
      .finally(() => this.#db.close());
  }

  /**
   * @param {bigint} n
   * @returns {Promise<void>}
   */
  setLastBlockHeight(n) {
    return this.getSyncInfo()
      .then(setKey("blockHeight", n))
      .then(async (syncInfo) => {
        await this.#db.open();

        return this.#db
          .transaction("rw", "syncInfo", async () => {
            const syncInfoTable = this.#db.table("syncInfo");

            await syncInfoTable.clear();
            await syncInfoTable.add(syncInfo);
          })
          .finally(() => this.#db.close());
      });
  }

  /**
   * @param {Uint8Array[]} nullifiers
   * @param {string} txId
   * @returns {Promise<void>}
   */
  async setPendingNoteInfo(nullifiers, txId) {
    const data = nullifiers.map((nullifier) => ({ nullifier, txId }));

    return this.#db
      .open()
      .then(async (db) => {
        await db.table("pendingNotesInfo").bulkAdd(data);
      })
      .finally(() => this.#db.close());
  }

  /**
   * @param {string} account
   * @param {StakeInfo} stakeInfo
   * @returns {Promise<void>}
   */
  async setStakeInfo(account, stakeInfo) {
    return this.#db
      .open()
      .then(async (db) => {
        await db.table("stakeInfo").put({ account, stakeInfo });
      })
      .finally(() => this.#db.close());
  }

  /**
   * @param {Uint8Array[]} nullifiers
   * @returns {Promise<void>}
   */
  async spendNotes(nullifiers) {
    await this.#db.open();

    return this.#db
      .transaction(
        "rw",
        ["pendingNotesInfo", "spentNotes", "unspentNotes"],
        async () => {
          const newlySpentNotes = await this.#db
            .table("unspentNotes")
            .where("nullifier")
            .anyOf(nullifiers)
            .toArray();

          await this.#db.table("pendingNotesInfo").bulkDelete(nullifiers);
          await this.#db.table("unspentNotes").bulkDelete(nullifiers);
          await this.#db.table("spentNotes").bulkAdd(newlySpentNotes);
        }
      )
      .finally(() => this.#db.close());
  }

  /**
   * @param {Uint8Array[]} nullifiers
   * @returns {Promise<void>}
   */
  async unspendNotes(nullifiers) {
    return this.#db
      .open()
      .then(async (db) =>
        db.transaction("rw", ["spentNotes", "unspentNotes"], async () => {
          const notesToUnspend = await db
            .table("spentNotes")
            .where("nullifier")
            .anyOf(nullifiers)
            .toArray();

          await this.#db.table("spentNotes").bulkDelete(nullifiers);
          await this.#db.table("unspentNotes").bulkAdd(notesToUnspend);
        })
      )
      .finally(() => this.#db.close());
  }

  /**
   * @param {Array<Map<Uint8Array, Uint8Array>>} syncerNotes
   * @param {Array<import("$lib/vendor/w3sper.js/src/mod").Profile>} profiles
   * @returns {WalletCacheNote[]}
   */
  toCacheNotes(syncerNotes, profiles) {
    const addresses = getAddressesFrom(profiles);

    return syncerNotes.reduce((result, entry, idx) => {
      Array.from(entry).forEach(([nullifier, note]) => {
        result.push({
          address: addresses[idx],
          note,
          nullifier,
        });
      });

      return result;
    }, /** @type {WalletCacheNote[]} */ ([]));
  }
}

export default new WalletCache();
