import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { mapValues, mapWith, multiplyBy, pluckFrom } from "lamb";

import mockedWalletStore from "$lib/mocks/mockedWalletStore";

import { cachePendingNotesInfo } from "$lib/mock-data";
import {
  fillCacheDatabase,
  getCacheDatabase,
  sortNullifiers,
} from "$lib/test-helpers";
import networkStore from "$lib/stores/networkStore";

import WalletTreasury from "..";
import walletCache from "$lib/wallet-cache";

describe("WalletTreasury", () => {
  /** @type {WalletTreasury} */
  let walletTreasury;

  const getBlockHashByHeightSpy = vi
    .spyOn(networkStore, "getBlockHashByHeight")
    .mockResolvedValue("fake-block-hash");
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
    getBlockHashByHeightSpy.mockClear();
  });

  afterAll(() => {
    getBlockHashByHeightSpy.mockRestore();
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

  describe("Cache access methods", () => {
    it("should expose a method to clear the local cache", async () => {
      // Right now we just call the cache method,
      // so we won't repeat the wallet-cache test here.
      const clearCacheSpy = vi
        .spyOn(walletCache, "clear")
        .mockResolvedValue(undefined);

      await walletTreasury.clearCache();

      expect(clearCacheSpy).toHaveBeenCalledTimes(1);

      clearCacheSpy.mockRestore();
    });

    it("should expose a method to get the cached balance", async () => {
      const expected = await walletCache.getBalanceInfo(
        profiles[0].address.toString()
      );

      await expect(
        walletTreasury.getCachedBalance(profiles[0])
      ).resolves.toStrictEqual(expected);
    });

    it("should expose a method to get the cached sync info", async () => {
      const expected = await walletCache.getSyncInfo();

      await expect(walletTreasury.getCachedSyncInfo()).resolves.toStrictEqual(
        expected
      );
    });

    it("should expose a method to get the cached stake info", async () => {
      const expected = await walletCache.getStakeInfo(
        profiles[0].address.toString()
      );

      await expect(
        walletTreasury.getCachedStakeInfo(profiles[0])
      ).resolves.toStrictEqual(expected);
    });

    it("should expose a method to cache the balance of a given profile", async () => {
      const address = profiles[0].address.toString();
      const currentBalance = await walletCache.getBalanceInfo(address);
      const newBalance = {
        shielded: mapValues(currentBalance.shielded, multiplyBy(2n)),
        unshielded: mapValues(currentBalance.unshielded, multiplyBy(3n)),
      };

      await walletTreasury.setCachedBalance(profiles[0], newBalance);

      await expect(walletCache.getBalanceInfo(address)).resolves.toStrictEqual(
        newBalance
      );
    });

    it("should expose a method to cache the stake info", async () => {
      // Right now we just call the cache method,
      // so we won't repeat the wallet-cache test here.
      const setCachedStakeInfoSpy = vi
        .spyOn(walletCache, "setStakeInfo")
        .mockResolvedValue(undefined);

      const fakeStakeInfo = {};

      // @ts-ignore we don't care to pass the correct type here
      await walletTreasury.setCachedStakeInfo(profiles[0], fakeStakeInfo);

      expect(setCachedStakeInfoSpy).toHaveBeenCalledTimes(1);
      expect(setCachedStakeInfoSpy).toHaveBeenCalledWith(
        profiles[0].account.toString(),
        fakeStakeInfo
      );

      setCachedStakeInfoSpy.mockRestore();
    });

    it("should expose a method to add notes to the pending notes cache", async () => {
      // Right now we just call the cache method,
      // so we won't repeat the wallet-cache test here.
      const setPendingNotesSpy = vi
        .spyOn(walletCache, "setPendingNotesInfo")
        .mockResolvedValue(undefined);

      const nullifiers = [new Uint8Array().fill(0)];
      const txHash = "some-tx-hash";

      await walletTreasury.updateCachedPendingNotes(nullifiers, txHash);

      expect(setPendingNotesSpy).toHaveBeenCalledTimes(1);
      expect(setPendingNotesSpy).toHaveBeenCalledWith(nullifiers, txHash);

      setPendingNotesSpy.mockRestore();
    });

    it("should expose a method to update the unshielded nonce", async () => {
      const address = profiles[0].address.toString();
      const currentlyCachedBalance = await walletCache.getBalanceInfo(address);
      const newNonce = currentlyCachedBalance.unshielded.nonce + 1n;

      await walletTreasury.updateCachedNonce(profiles[0], newNonce);

      await expect(walletCache.getBalanceInfo(address)).resolves.toStrictEqual({
        ...currentlyCachedBalance,
        unshielded: {
          ...currentlyCachedBalance.unshielded,
          nonce: newNonce,
        },
      });
    });
  });
});
