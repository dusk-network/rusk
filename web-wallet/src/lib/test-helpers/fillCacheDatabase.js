import { getCacheDatabase } from ".";

import {
  cacheHistory,
  cacheSpentNotes,
  cacheUnspentNotes,
} from "$lib/mock-data";

/** @type {() => Promise<void>} */
async function fillCacheDatabase() {
  const db = getCacheDatabase();

  await db.open();

  return db
    .transaction("rw", ["history", "spentNotes", "unspentNotes"], async () => {
      await db.table("history").bulkPut(cacheHistory);
      await db.table("spentNotes").bulkPut(cacheSpentNotes);
      await db.table("unspentNotes").bulkPut(cacheUnspentNotes);
    })
    .finally(() => {
      db.close();
    });
}

export default fillCacheDatabase;
