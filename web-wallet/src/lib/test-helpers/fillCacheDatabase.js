import { getCacheDatabase } from ".";

import {
  cachePendingNotesInfo,
  cacheSpentNotes,
  cacheSyncInfo,
  cacheUnspentNotes,
} from "$lib/mock-data";

/** @type {() => Promise<void>} */
async function fillCacheDatabase() {
  const db = getCacheDatabase();

  await db.open();

  return db
    .transaction(
      "rw",
      ["pendingNotesInfo", "spentNotes", "syncInfo", "unspentNotes"],
      async () => {
        await db.table("pendingNotesInfo").bulkPut(cachePendingNotesInfo);
        await db.table("spentNotes").bulkPut(cacheSpentNotes);
        await db.table("syncInfo").bulkPut(cacheSyncInfo);
        await db.table("unspentNotes").bulkPut(cacheUnspentNotes);
      }
    )
    .finally(() => {
      db.close();
    });
}

export default fillCacheDatabase;
