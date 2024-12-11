import { Dexie } from "dexie";

/** @type {() => Dexie} */
function getCacheDatabase() {
  const db = new Dexie("@dusk-network/wallet-cache");

  db.version(3).stores({
    balancesInfo: "address",
    pendingNotesInfo: "nullifier,txId",
    spentNotes: "nullifier,address",
    stakeInfo: "account",
    syncInfo: "++",
    unspentNotes: "nullifier,address",
  });

  return db;
}

export default getCacheDatabase;
