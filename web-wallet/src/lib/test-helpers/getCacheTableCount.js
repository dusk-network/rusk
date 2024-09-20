import { getCacheDatabase } from ".";

/** @type {(tableName: WalletCacheTableName) => Promise<number>} */
async function getCacheTableCount(tableName) {
  const db = await getCacheDatabase().open();

  return db
    .table(tableName)
    .count()
    .finally(() => db.close());
}

export default getCacheTableCount;
