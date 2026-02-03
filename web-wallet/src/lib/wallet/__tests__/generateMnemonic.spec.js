import { describe, expect, it } from "vitest";

import { generateMnemonic } from "..";

describe("generateMnemonic", () => {
  it("should generate a twelve word mnemonic", () => {
    const mnemonic = generateMnemonic();

    expect(mnemonic).toMatch(/^(?:[a-z]+ ){11}[a-z]+$/);
    expect(generateMnemonic()).not.toBe(mnemonic);
  });
});
