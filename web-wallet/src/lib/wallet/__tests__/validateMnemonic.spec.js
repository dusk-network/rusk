import { describe, expect, it } from "vitest";
import { last, setIndex } from "lamb";

import { generateMnemonic, validateMnemonic } from "..";

describe("validateMnemonic", () => {
  it("should correctly validate a generated mnemonic", () => {
    expect(validateMnemonic(generateMnemonic())).toBe(true);
  });

  it("should return false for an invalid mnemonic", () => {
    expect(validateMnemonic("pizza pasta mandolino")).toBe(false);
    expect(validateMnemonic(Array(12).fill("auction").join(" "))).toBe(false);

    const validMnemonic = generateMnemonic();
    const words = validMnemonic.split(" ");
    const originalLastWord = last(words);
    const fakeLastWord = originalLastWord === "zoo" ? "abandon" : "zoo";
    const tamperedMnemonic = setIndex(words, -1, fakeLastWord).join(" ");

    expect(validateMnemonic(tamperedMnemonic)).toBe(false);
  });

  it("should return false for empty or partial input", () => {
    expect(validateMnemonic("")).toBe(false);
    expect(validateMnemonic("apple")).toBe(false);
  });
});
