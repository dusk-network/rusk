import { beforeEach, describe, expect, it } from "vitest";

import {
  cachePendingNotesInfo,
  cacheSpentNotes,
  cacheSyncInfo,
  cacheUnspentNotes,
} from "$lib/mock-data";

import { fillCacheDatabase, getCacheDatabase, sortByNullifier } from "..";

describe("fillCacheDatabase", () => {
  beforeEach(async () => {
    await getCacheDatabase().delete();
  });

  it("should fill the database tables with mock data", async () => {
    const expectedPendingNotesInfo = sortByNullifier(cachePendingNotesInfo);
    const expectedSpentNotes = sortByNullifier(cacheSpentNotes);
    const expectedUnspentNotes = sortByNullifier(cacheUnspentNotes);

    await fillCacheDatabase();

    const db = getCacheDatabase();

    await db.open();

    await expect(
      db.table("pendingNotesInfo").toArray().then(sortByNullifier)
    ).resolves.toStrictEqual(expectedPendingNotesInfo);
    await expect(
      db.table("spentNotes").toArray().then(sortByNullifier)
    ).resolves.toStrictEqual(expectedSpentNotes);
    await expect(db.table("syncInfo").toArray()).resolves.toStrictEqual(
      cacheSyncInfo
    );
    await expect(
      db.table("unspentNotes").toArray().then(sortByNullifier)
    ).resolves.toStrictEqual(expectedUnspentNotes);

    db.close();
  });
});
