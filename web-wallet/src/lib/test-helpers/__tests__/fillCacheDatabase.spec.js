import { describe, expect, it } from "vitest";
import { getKey, sortWith } from "lamb";

import {
  cacheHistory,
  cacheSpentNotes,
  cacheUnspentNotes,
} from "$lib/mock-data";

import { fillCacheDatabase, getCacheDatabase, sortCacheNotes } from "..";

const sortHistory = sortWith([getKey("id")]);

describe("fillCacheDatabase", () => {
  it("should fill the database tables with mock data", async () => {
    const expectedHistory = sortHistory(cacheHistory);
    const expectedSpentNotes = sortCacheNotes(cacheSpentNotes);
    const expectedUnspentNotes = sortCacheNotes(cacheUnspentNotes);

    await fillCacheDatabase();

    const db = getCacheDatabase();

    await db.open();

    await expect(
      db.table("history").toArray().then(sortHistory)
    ).resolves.toStrictEqual(expectedHistory);
    await expect(
      db.table("spentNotes").toArray().then(sortCacheNotes)
    ).resolves.toStrictEqual(expectedSpentNotes);
    await expect(
      db.table("unspentNotes").toArray().then(sortCacheNotes)
    ).resolves.toStrictEqual(expectedUnspentNotes);

    db.close();
  });
});
