import { describe, expect, it } from "vitest";
import { getKey } from "lamb";

import { getCacheDatabase } from "..";

describe("getCacheDatabase", () => {
  it("should return the Dexie database used for the cache", async () => {
    const db = getCacheDatabase();

    await db.open();

    expect(db.tables.map(getKey("name"))).toMatchInlineSnapshot(`
      [
        "history",
        "spentNotes",
        "unspentNotes",
      ]
    `);

    db.close();
  });
});
