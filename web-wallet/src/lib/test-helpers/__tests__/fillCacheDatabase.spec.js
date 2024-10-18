import { beforeEach, describe, expect, it } from "vitest";
import { sortWith } from "lamb";

import {
  cachePendingNotesInfo,
  cacheSpentNotes,
  cacheSyncInfo,
  cacheUnspentNotes,
} from "$lib/mock-data";

import { fillCacheDatabase, getCacheDatabase, sortByNullifier } from "..";

/**
 * We need to sort the entries in tests as the
 * database doesn't guarantee a sort order.
 *
 * @typedef {{ nullifier: ArrayBuffer }} T
 * @type {<U extends T>(entries: U[]) => U[]}
 */
const sortByDbNullifier = sortWith([
  /** @type {(entry: T) => string} */ (
    ({ nullifier }) => new Uint8Array(nullifier).toString()
  ),
]);

/** @type {(entry: WalletCacheNote) => WalletCacheDbNote} */
const toDbNote = (entry) => ({
  ...entry,
  note: entry.note.buffer,
  nullifier: entry.nullifier.buffer,
});

describe("fillCacheDatabase", () => {
  beforeEach(async () => {
    await getCacheDatabase().delete({ disableAutoOpen: false });
  });

  it("should fill the database tables with mock data", async () => {
    const expectedPendingNotesInfo = sortByNullifier(cachePendingNotesInfo).map(
      (v) => ({
        ...v,
        nullifier: v.nullifier.buffer,
      })
    );
    const expectedSpentNotes = sortByNullifier(cacheSpentNotes).map(toDbNote);
    const expectedUnspentNotes =
      sortByNullifier(cacheUnspentNotes).map(toDbNote);

    await fillCacheDatabase();

    const db = getCacheDatabase();

    await db.open();

    await expect(
      db.table("pendingNotesInfo").toArray().then(sortByDbNullifier)
    ).resolves.toStrictEqual(expectedPendingNotesInfo);
    await expect(
      db.table("spentNotes").toArray().then(sortByDbNullifier)
    ).resolves.toStrictEqual(expectedSpentNotes);
    await expect(db.table("syncInfo").toArray()).resolves.toStrictEqual(
      cacheSyncInfo
    );
    await expect(
      db.table("unspentNotes").toArray().then(sortByDbNullifier)
    ).resolves.toStrictEqual(expectedUnspentNotes);

    db.close();
  });
});
