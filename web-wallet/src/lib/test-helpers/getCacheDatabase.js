import { Dexie } from "dexie";

/** @type {() => Dexie} */
function getCacheDatabase() {
  const db = new Dexie("@dusk-network/wallet-cache");

  db.version(1).stores({
    pendingNotesInfo: "nullifier,txId",
    spentNotes: "nullifier,address",
    syncInfo: "++",
    unspentNotes: "nullifier,address",
  });

  return db;
}

export default getCacheDatabase;
