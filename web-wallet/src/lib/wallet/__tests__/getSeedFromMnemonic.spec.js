import { describe, expect, it } from "vitest";
import { generateMnemonic } from "bip39";
import { getSeedFromMnemonic } from "..";

describe("getSeedFromMnemonic", () => {
  it("should convert a mnemonic phrase into a seed of 64 bytes", () => {
    const mnemonic = generateMnemonic();
    const seed = getSeedFromMnemonic(mnemonic);

    expect(seed).toBeInstanceOf(Uint8Array);
    expect(seed.byteLength).toBe(64);
  });
});
