import { describe, expect, it } from "vitest";
import { getKey } from "lamb";

import { fillCacheDatabase, getCacheDatabase, getCacheTableCount } from "..";

describe("getCacheTableCount", () => {
  it("should return the amount of items in a database table", async () => {
    const db = getCacheDatabase();

    await db.open();

    const tableNames = /** @type {WalletCacheTableName[]} */ (
      db.tables.map(getKey("name"))
    );

    for (const tableName of tableNames) {
      await expect(getCacheTableCount(tableName)).resolves.toBe(0);
    }

    await fillCacheDatabase();

    for (const tableName of tableNames) {
      await expect(getCacheTableCount(tableName)).resolves.toBe(
        await db.table(tableName).count()
      );
    }

    db.close();
  });
});
