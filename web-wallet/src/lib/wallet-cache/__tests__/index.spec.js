import { beforeEach, describe, expect, it } from "vitest";
import { filterWith, pluckFrom, setKey, takeFrom, uniques } from "lamb";

import {
  cachePendingNotesInfo,
  cacheSpentNotes,
  cacheSyncInfo,
  cacheUnspentNotes,
} from "$lib/mock-data";
import {
  fillCacheDatabase,
  getCacheDatabase,
  sortByNullifier,
  sortNullifiers,
} from "$lib/test-helpers";

import walletCache from "..";

describe("Wallet cache", () => {
  const db = getCacheDatabase();

  beforeEach(async () => {
    await getCacheDatabase().delete();
    await fillCacheDatabase();
  });

  describe("Reading and clearing the cache", async () => {
    const addresses = takeFrom(
      uniques(pluckFrom(cacheSpentNotes, "address")),
      2
    );
    const addressA = addresses[0];

    /** @type {(entries: WalletCacheNote[]) => WalletCacheNote[]} */
    const filterByAddressA = filterWith((entry) => entry.address === addressA);

    /** @type {(entries: WalletCacheNote[]) => WalletCacheNote[]} */
    const filterByAddresses = filterWith((entry) =>
      addresses.includes(entry.address)
    );

    it("should expose a method to clear the database", async () => {
      await db.open();

      for (const table of db.tables) {
        await expect(table.count()).resolves.toBeGreaterThan(0);
      }

      /**
       * Closing the db to suppress the warning about another
       * connection wanting to delete the database.
       */
      db.close();

      await walletCache.clear();

      await db.open();

      for (const table of db.tables) {
        await expect(table.count()).resolves.toBe(0);
      }

      db.close();
    });

    it("should expose a method to retrieve the pending notes info and optionally filter them by their nullifiers", async () => {
      const pendingDbNotesInfo = sortByNullifier(
        await walletCache.getPendingNotesInfo()
      );
      const nullifiers = takeFrom(
        pluckFrom(pendingDbNotesInfo, "nullifier"),
        2
      );
      const pendingNotesInfo = sortByNullifier(cachePendingNotesInfo);
      const pendingDbNotesInfoByNullifiers = sortByNullifier(
        await walletCache.getPendingNotesInfo(nullifiers)
      );
      const pendingNotesInfoByNullifiers = takeFrom(pendingNotesInfo, 2);

      expect(pendingDbNotesInfo).toStrictEqual(pendingNotesInfo);
      expect(pendingDbNotesInfoByNullifiers).toStrictEqual(
        pendingNotesInfoByNullifiers
      );
      await expect(
        walletCache.getPendingNotesInfo([Uint8Array.of(1, 2, 3)])
      ).resolves.toStrictEqual([]);
    });

    it("should expose a method to retrieve the spent notes and optionally filter them by their address", async () => {
      const spentDbNotes = sortByNullifier(await walletCache.getSpentNotes());
      const spentNotes = sortByNullifier(cacheSpentNotes);
      const spentDbNotesByAddressA = sortByNullifier(
        await walletCache.getSpentNotes([addressA])
      );
      const spentDbNotesByAddresses = sortByNullifier(
        await walletCache.getSpentNotes(addresses)
      );
      const spentNotesByAddressA = filterByAddressA(spentNotes);
      const spentNotesByAddresses = filterByAddresses(spentNotes);

      expect(spentDbNotes).toStrictEqual(spentNotes);
      expect(spentDbNotesByAddresses).toStrictEqual(spentNotesByAddresses);
      expect(
        sortByNullifier(await walletCache.getSpentNotes([]))
      ).toStrictEqual(spentNotes);
      expect(spentDbNotesByAddressA).toStrictEqual(spentNotesByAddressA);
      await expect(walletCache.getSpentNotes(["foo"])).resolves.toStrictEqual(
        []
      );
    });

    it("should expose a method to retrieve the spent notes nullifiers and optionally filter them by their address", async () => {
      const spentDbNullifiers = sortNullifiers(
        await walletCache.getSpentNotesNullifiers()
      );
      const spentNullifiers = sortNullifiers(
        pluckFrom(cacheSpentNotes, "nullifier")
      );
      const spentDbNullifiersByAddressA = sortNullifiers(
        await walletCache.getSpentNotesNullifiers([addressA])
      );
      const spentDbNullifiersByAddresses = sortNullifiers(
        await walletCache.getSpentNotesNullifiers(addresses)
      );
      const spentNullifiersByAddressA = pluckFrom(
        sortByNullifier(filterByAddressA(cacheSpentNotes)),
        "nullifier"
      );
      const spentNullifiersByAddresses = pluckFrom(
        sortByNullifier(filterByAddresses(cacheSpentNotes)),
        "nullifier"
      );

      expect(spentDbNullifiers).toStrictEqual(spentNullifiers);
      expect(spentDbNullifiersByAddresses).toStrictEqual(
        spentNullifiersByAddresses
      );
      expect(
        sortNullifiers(await walletCache.getSpentNotesNullifiers([]))
      ).toStrictEqual(spentNullifiers);
      expect(spentDbNullifiersByAddressA).toStrictEqual(
        spentNullifiersByAddressA
      );
      await expect(
        walletCache.getSpentNotesNullifiers(["foo"])
      ).resolves.toStrictEqual([]);
    });

    it("should expose a method to retrieve the sync info, which returns `{ blockHeight: 0n, bookmark: 0n }` if there is no info stored", async () => {
      await expect(walletCache.getSyncInfo()).resolves.toStrictEqual(
        cacheSyncInfo[0]
      );

      await walletCache.clear();

      await expect(walletCache.getSyncInfo()).resolves.toStrictEqual({
        blockHeight: 0n,
        bookmark: 0n,
      });
    });

    it("should expose a method to retrieve the unspent notes and optionally filter them by a `psk`", async () => {
      const unspentDbNotes = sortByNullifier(
        await walletCache.getUnspentNotes()
      );
      const unspentNotes = sortByNullifier(cacheUnspentNotes);
      const unspentDbNotesByAddressA = sortByNullifier(
        await walletCache.getUnspentNotes([addressA])
      );
      const unspentDbNotesByAddresses = sortByNullifier(
        await walletCache.getUnspentNotes(addresses)
      );
      const unspentNotesByAddressA = filterByAddressA(unspentNotes);
      const unspentNotesByAddresses = filterByAddresses(unspentNotes);

      expect(unspentDbNotes).toStrictEqual(unspentNotes);
      expect(unspentDbNotesByAddresses).toStrictEqual(unspentNotesByAddresses);
      expect(
        sortByNullifier(await walletCache.getUnspentNotes([]))
      ).toStrictEqual(unspentNotes);
      expect(unspentDbNotesByAddressA).toStrictEqual(unspentNotesByAddressA);
      await expect(walletCache.getUnspentNotes(["foo"])).resolves.toStrictEqual(
        []
      );
    });

    it("should expose a method to retrieve the unspent notes nullifiers and optionally filter them by their address", async () => {
      const unspentDbNullifiers = sortNullifiers(
        await walletCache.getUnspentNotesNullifiers()
      );
      const unspentNullifiers = sortNullifiers(
        pluckFrom(cacheUnspentNotes, "nullifier")
      );
      const unspentDbNullifiersByAddressA = sortNullifiers(
        await walletCache.getUnspentNotesNullifiers([addressA])
      );
      const unspentDbNullifiersByAddresses = sortNullifiers(
        await walletCache.getUnspentNotesNullifiers(addresses)
      );
      const unspentNullifiersByAddressA = pluckFrom(
        sortByNullifier(filterByAddressA(cacheUnspentNotes)),
        "nullifier"
      );
      const unspentNullifiersByAddresses = pluckFrom(
        sortByNullifier(filterByAddresses(cacheUnspentNotes)),
        "nullifier"
      );

      expect(unspentDbNullifiers).toStrictEqual(unspentNullifiers);
      expect(unspentDbNullifiersByAddresses).toStrictEqual(
        unspentNullifiersByAddresses
      );
      expect(
        sortNullifiers(await walletCache.getUnspentNotesNullifiers([]))
      ).toStrictEqual(unspentNullifiers);
      expect(unspentDbNullifiersByAddressA).toStrictEqual(
        unspentNullifiersByAddressA
      );
      await expect(
        walletCache.getUnspentNotesNullifiers(["foo"])
      ).resolves.toStrictEqual([]);
    });
  });

  describe("Adding notes", () => {
    const address = cacheUnspentNotes[0].address;

    /** @type {(note: WalletCacheNote) => boolean} */
    const hasTestAddress = (note) => note.address === address;

    /** @type {WalletCacheNote} */
    const newNote = {
      address,
      note: new Uint8Array(),
      nullifier: new Uint8Array(32).fill(0),
    };

    it("should expose a method to add new notes to the unspent list", async () => {
      /*
       * We just pick some notes to add from the spent list for the test,
       * as we just need to see that they are added.
       * Notes can't go from spent to unspent anyway.
       */
      const unspentNotesToAdd = cacheSpentNotes.map(setKey("address", address));

      const unspentNoteDuplicate = cacheUnspentNotes.find(hasTestAddress);

      if (!unspentNoteDuplicate) {
        throw new Error(
          "No suitable unspent note found to setup the duplicate test"
        );
      }

      /*
       * We also pick an existing unspent note to verify
       * that duplicates aren't being added.
       */
      const newNotes = unspentNotesToAdd.concat(
        newNote,
        structuredClone(unspentNoteDuplicate)
      );
      const expectedUnspentNotes = cacheUnspentNotes.concat(
        newNote,
        unspentNotesToAdd
      );

      await walletCache.addUnspentNotes(newNotes);

      await expect(
        walletCache.getUnspentNotes().then(sortByNullifier)
      ).resolves.toStrictEqual(sortByNullifier(expectedUnspentNotes));
    });

    it("should leave the unspent notes as they were if an error occurs during insertion", async () => {
      // @ts-expect-error
      const newNotes = cacheSpentNotes.concat({});

      await expect(
        walletCache.addUnspentNotes(newNotes)
      ).rejects.toBeInstanceOf(Error);

      expect(sortByNullifier(cacheUnspentNotes)).toStrictEqual(
        sortByNullifier(await walletCache.getUnspentNotes())
      );
    });
  });
});
