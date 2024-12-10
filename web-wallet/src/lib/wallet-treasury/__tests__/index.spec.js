import { afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { mapWith, pluckFrom } from "lamb";

import mockedWalletStore from "../../../__mocks__/mockedWalletStore";

import { cachePendingNotesInfo } from "$lib/mock-data";
import {
  fillCacheDatabase,
  getCacheDatabase,
  sortNullifiers,
} from "$lib/test-helpers";

import WalletTreasury from "..";

describe("WalletTreasury", () => {
  /** @type {WalletTreasury} */
  let walletTreasury;

  const { profiles } = mockedWalletStore.getMockedStoreValue();
  const db = getCacheDatabase();

  beforeAll(async () => {
    await fillCacheDatabase();
  });

  beforeEach(() => {
    walletTreasury = new WalletTreasury(profiles);
  });

  afterEach(async () => {
    await getCacheDatabase().delete({ disableAutoOpen: false });
    await fillCacheDatabase();
  });

  describe("Treasury interface", () => {
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
            !pendingNullifiersAsStrings.includes(
              new Uint8Array(note.nullifier).toString()
            )
        )
        .toArray()
        .then(
          mapWith((entry) => ({
            ...entry,
            note: new Uint8Array(entry.note),
            nullifier: new Uint8Array(entry.nullifier),
          }))
        );

      db.close();
    });

    it("should implement the `account` method of the treasury interface to retrieve the moonlight account balance for a given identifier", async () => {
      const abortController = new AbortController();

      await walletTreasury.update(0n, () => {}, abortController.signal);

      // @ts-expect-error We don't care to pass a real `Key` object right now
      await expect(walletTreasury.account(fakeKey)).resolves.toStrictEqual(
        expect.objectContaining({
          nonce: expect.any(BigInt),
          value: expect.any(BigInt),
        })
      );
    });

    it("should return a rejected promise if the `account` method isn't able to find the balance for the given identifier", async () => {
      // @ts-expect-error We don't care to pass a real `Key` object
      await expect(walletTreasury.account(fakeKey)).rejects.toThrow();
    });

    it("should implement the `address` method of the treasury interface to retrieve all non-pending unspent notes for a given identifier", async () => {
      // @ts-expect-error We don't care to pass a real `Key` object
      const unspentNotesMapA = await walletTreasury.address(fakeKey);

      // @ts-expect-error We don't care to pass a real `Key` object
      const unspentNotesMapB = await walletTreasury.address({
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

    it("should implement the `stakeInfo` method of the treasury interface to retrieve the stake info for a given account", async () => {
      const abortController = new AbortController();

      await walletTreasury.update(0n, () => {}, abortController.signal);

      // @ts-expect-error We don't care to pass a real `Key` object right now
      await expect(walletTreasury.stakeInfo(fakeKey)).resolves.toStrictEqual(
        expect.objectContaining({
          amount: expect.objectContaining({
            eligibility: expect.any(BigInt),
            locked: expect.any(BigInt),
            total: expect.any(BigInt),
            value: expect.any(BigInt),
          }),
          faults: expect.any(Number),
          hardFaults: expect.any(Number),
          reward: expect.any(BigInt),
        })
      );
    });

    it("should return a rejected promise if the `stakeInfo` method isn't able to find the stake info for the given account", async () => {
      await expect(
        // @ts-expect-error We don't care to pass a real `Key` object right now
        walletTreasury.stakeInfo({
          toString() {
            return "non-existent address";
          },
        })
      ).rejects.toThrow();
    });
  });
});
