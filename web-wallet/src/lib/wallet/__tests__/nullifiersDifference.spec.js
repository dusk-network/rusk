import { describe, expect, it } from "vitest";
import { pluck } from "lamb";

import { cacheUnspentNotes } from "$lib/mock-data";

import { nullifiersDifference } from "..";

/** @type {(source: WalletCacheNote[]) => Uint8Array[]} */
const getNullifiers = pluck("nullifier");

describe("nullifiersDifference", () => {
  it("should return the array of unique nullifiers contained only in the first of the two given sets of nullifiers", () => {
    const a = getNullifiers(cacheUnspentNotes);
    const b = getNullifiers(cacheUnspentNotes.slice(0, a.length - 2));

    // ensure we have meaningful data for the test
    expect(a.length).toBeGreaterThan(0);
    expect(b.length).toBeGreaterThan(1);

    expect(nullifiersDifference(a, b)).toStrictEqual(a.slice(-2));
    expect(nullifiersDifference(b, a)).toStrictEqual([]);
    expect(nullifiersDifference(a, [])).toStrictEqual(a);
    expect(nullifiersDifference([], b)).toStrictEqual([]);
  });
});
