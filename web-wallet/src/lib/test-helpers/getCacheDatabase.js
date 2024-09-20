import { Dexie } from "dexie";

/** @type {() => Dexie} */
function getCacheDatabase() {
  const db = new Dexie("@dusk-network/wallet-cache");

  db.version(1).stores({
    history: "psk",
    spentNotes: "nullifier,&pos,psk",
    unspentNotes: "nullifier,&pos,psk",
  });

  return db;
}

export default getCacheDatabase;
