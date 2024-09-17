import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import {
  add,
  collect,
  compose,
  drop,
  filterWith,
  getKey,
  mapValues,
  partitionWith,
  pluckFrom,
  setKey,
  take,
  takeFrom,
  uniques,
} from "lamb";

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
import notesArrayToMap from "$lib/wallet/notesArrayToMap";

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

    it("should expose a method to retrieve the unspent notes and optionally filter them by their address", async () => {
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
    /** @type {WalletCacheSyncInfo} */
    let currentSyncInfo;

    /** @type {WalletCacheSyncInfo} */
    let newSyncInfo;

    const address = cacheUnspentNotes[0].address;

    /** @type {(note: WalletCacheNote) => boolean} */
    const hasTestAddress = (note) => note.address === address;

    /** @type {WalletCacheNote} */
    const newNote = {
      address,
      note: new Uint8Array(),
      nullifier: new Uint8Array(32).fill(0),
    };

    beforeEach(async () => {
      currentSyncInfo = await walletCache.getSyncInfo();

      expect(currentSyncInfo.blockHeight).toBeGreaterThan(0n);
      expect(currentSyncInfo.bookmark).toBeGreaterThan(0n);

      newSyncInfo = mapValues(currentSyncInfo, add(999n));
    });

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

      await walletCache.addUnspentNotes(newNotes, newSyncInfo);

      await expect(
        walletCache.getUnspentNotes().then(sortByNullifier)
      ).resolves.toStrictEqual(sortByNullifier(expectedUnspentNotes));
      await expect(walletCache.getSyncInfo()).resolves.toStrictEqual(
        newSyncInfo
      );
    });

    it("should leave the unspent notes and the sync info as they were if an error occurs during insertion", async () => {
      // @ts-expect-error
      const newNotes = cacheSpentNotes.concat({});

      await expect(
        walletCache.addUnspentNotes(newNotes, newSyncInfo)
      ).rejects.toBeInstanceOf(Error);

      expect(sortByNullifier(cacheUnspentNotes)).toStrictEqual(
        sortByNullifier(await walletCache.getUnspentNotes())
      );
      await expect(walletCache.getSyncInfo()).resolves.toStrictEqual(
        currentSyncInfo
      );
    });
  });

  describe("Spending notes", () => {
    it("should expose a method to move a group of notes from the unspent to the spent table", async () => {
      /** @type {(entry: { nullifier: Uint8Array }) => Uint8Array} */
      const getNullifier = getKey("nullifier");
      const [pendingToSpend, expectedPending] = await walletCache
        .getPendingNotesInfo()
        .then(sortByNullifier)
        .then(collect([take(2), drop(2)]));

      // checks to ensure we have enough meaningful data for the test
      expect(pendingToSpend.length).toBe(2);
      expect(expectedPending.length).toBeGreaterThan(0);

      const pendingNullifiersLookup = new Set(
        pendingToSpend.map(compose(String, getNullifier))
      );

      const [expectedNotesToBeSpent, expectedUnspentNotes] = await walletCache
        .getUnspentNotes()
        .then(sortByNullifier)
        .then(
          partitionWith((note) =>
            pendingNullifiersLookup.has(note.nullifier.toString())
          )
        );

      const expectedSpentNotes = await walletCache
        .getSpentNotes()
        .then((notes) => notes.concat(expectedNotesToBeSpent))
        .then(sortByNullifier);

      // checks to ensure we have enough meaningful data for the test
      expect(expectedNotesToBeSpent.length).toBeGreaterThan(0);
      expect(expectedUnspentNotes.length).toBeGreaterThan(0);
      expect(expectedSpentNotes.length).toBeGreaterThan(0);

      const nullifiersToSpend = pendingToSpend
        .map(getNullifier)
        .concat(expectedNotesToBeSpent.map(getNullifier));

      await walletCache.spendNotes(nullifiersToSpend);

      await expect(
        walletCache.getUnspentNotes().then(sortByNullifier)
      ).resolves.toStrictEqual(expectedUnspentNotes);
      await expect(
        walletCache.getPendingNotesInfo().then(sortByNullifier)
      ).resolves.toStrictEqual(expectedPending);
      await expect(
        walletCache.getSpentNotes().then(sortByNullifier)
      ).resolves.toStrictEqual(expectedSpentNotes);
    });

    it("should leave the database as is if the array of nullifiers to spend is empty", async () => {
      const currentPendingInfo = await walletCache.getPendingNotesInfo();
      const currentSpentNotes = await walletCache.getSpentNotes();
      const currentUnspentNotes = await walletCache.getUnspentNotes();

      await walletCache.spendNotes([]);

      await expect(walletCache.getPendingNotesInfo()).resolves.toStrictEqual(
        currentPendingInfo
      );
      await expect(walletCache.getSpentNotes()).resolves.toStrictEqual(
        currentSpentNotes
      );
      await expect(walletCache.getUnspentNotes()).resolves.toStrictEqual(
        currentUnspentNotes
      );
    });

    it("should leave the database as is if an error occurs during the spend procedure", async () => {
      const currentPendingInfo = await walletCache.getPendingNotesInfo();
      const currentSpentNotes = await walletCache.getSpentNotes();
      const currentUnspentNotes = await walletCache.getUnspentNotes();

      // @ts-expect-error We are passing an invalid value on purpose
      await walletCache.spendNotes(() => {});

      await expect(walletCache.getPendingNotesInfo()).resolves.toStrictEqual(
        currentPendingInfo
      );
      await expect(walletCache.getSpentNotes()).resolves.toStrictEqual(
        currentSpentNotes
      );
      await expect(walletCache.getUnspentNotes()).resolves.toStrictEqual(
        currentUnspentNotes
      );
    });
  });

  describe("Treasury", async () => {
    /** @type {string} */
    let addressWithPendingNotes;

    /** @type {WalletCacheNote[]} */
    let expectedNotes;

    const nullifierToExclude = cachePendingNotesInfo[1].nullifier;
    const pendingNullifiersAsStrings = pluckFrom(
      cachePendingNotesInfo,
      "nullifier"
    ).map(String);
    const fakeKey = {
      toString() {
        return addressWithPendingNotes;
      },
    };

    beforeAll(async () => {
      await db.open();

      addressWithPendingNotes = (
        await db
          .table("unspentNotes")
          .where("nullifier")
          .equals(nullifierToExclude)
          .first()
      )?.address;

      if (!addressWithPendingNotes) {
        throw new Error(
          "A pending note with a nullifier present in unspent notes is missing"
        );
      }

      expectedNotes = await db
        .table("unspentNotes")
        .where("address")
        .equals(addressWithPendingNotes)
        .and(
          (note) =>
            !pendingNullifiersAsStrings.includes(note.nullifier.toString())
        )
        .toArray();

      db.close();
    });

    it("should expose a treasury interface that allows to retrieve all non-pending unspent notes for a given profile", async () => {
      // @ts-expect-error We don't care to pass a real `Key` object
      const unspentNotesMapA = await walletCache.treasury.address(fakeKey);

      // @ts-expect-error We don't care to pass a real `Key` object
      const unspentNotesMapB = await walletCache.treasury.address({
        toString() {
          return "non-existent address";
        },
      });

      expect(expectedNotes.length).toBe(unspentNotesMapA.size);
      expect(sortNullifiers(Array.from(unspentNotesMapA.keys()))).toStrictEqual(
        sortNullifiers(pluckFrom(expectedNotes, "nullifier"))
      );
      expect(
        sortNullifiers(Array.from(unspentNotesMapA.values()))
      ).toStrictEqual(sortNullifiers(pluckFrom(expectedNotes, "note")));
      expect(unspentNotesMapB).toStrictEqual(new Map());
    });
  });

  describe("Utilities", () => {
    it("should expose a method to update the last block height", async () => {
      const currentSyncInfo = await walletCache.getSyncInfo();
      const newBlockHeight = currentSyncInfo.blockHeight * 2n;

      await walletCache.setLastBlockHeight(newBlockHeight);

      await expect(walletCache.getSyncInfo()).resolves.toStrictEqual({
        ...currentSyncInfo,
        blockHeight: newBlockHeight,
      });
    });

    it("should leave the last block height as it is if an error occurs while writing the new value", async () => {
      const currentSyncInfo = await walletCache.getSyncInfo();

      // @ts-expect-error We are passing an invalid value on purpose
      await expect(walletCache.setLastBlockHeight(() => {})).rejects.toThrow();

      await expect(walletCache.getSyncInfo()).resolves.toStrictEqual(
        currentSyncInfo
      );
    });

    it("should expose a method to convert notes in the w3sper map format into the one used by the cache", () => {
      const addresses = uniques(pluckFrom(cacheUnspentNotes, "address"));
      const fakeProfiles = addresses.map((address) => ({
        address: {
          toString() {
            return address;
          },
        },
      }));
      const notesAsMap = notesArrayToMap(cacheUnspentNotes);
      const notesArray = /** @type {Array<Map<Uint8Array, Uint8Array>>} */ (
        addresses.map((address) => notesAsMap.get(address))
      );

      expect(
        // @ts-expect-error we are passing fake profiles
        sortByNullifier(walletCache.toCacheNotes(notesArray, fakeProfiles))
      ).toStrictEqual(sortByNullifier(cacheUnspentNotes));
      expect(walletCache.toCacheNotes([], [])).toStrictEqual([]);
    });
  });
});
