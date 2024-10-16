import { mapWith } from "lamb";
import { getCacheDatabase } from ".";

import {
  cachePendingNotesInfo,
  cacheSpentNotes,
  cacheSyncInfo,
  cacheUnspentNotes,
} from "$lib/mock-data";

/**
 * In IndexedDB if we write a Uint8Array, we get
 * back an ArrayBuffer when we retrieve the data.
 *
 * In `fake-indexeddb` this is not the case, so
 * we intentionally write ArrayBuffers from the
 * beginning.
 */
const fixPending = mapWith((record) => ({
  ...record,
  nullifier: record.nullifier.buffer,
}));
const fixNotes = mapWith((record) => ({
  ...record,
  note: record.note.buffer,
  nullifier: record.nullifier.buffer,
}));

/** @type {() => Promise<void>} */
async function fillCacheDatabase() {
  const db = getCacheDatabase();

  await db.open();

  return db
    .transaction(
      "rw",
      ["pendingNotesInfo", "spentNotes", "syncInfo", "unspentNotes"],
      async () => {
        await db
          .table("pendingNotesInfo")
          .bulkPut(fixPending(cachePendingNotesInfo));
        await db.table("spentNotes").bulkPut(fixNotes(cacheSpentNotes));
        await db.table("syncInfo").bulkPut(cacheSyncInfo);
        await db.table("unspentNotes").bulkPut(fixNotes(cacheUnspentNotes));
      }
    )
    .finally(() => {
      db.close();
    });
}

export default fillCacheDatabase;
