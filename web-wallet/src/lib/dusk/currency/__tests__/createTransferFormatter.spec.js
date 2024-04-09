import { describe, expect, it } from "vitest";

import { createTransferFormatter } from "..";

describe("createTransferFormatter", () => {
  it("should create a locale aware formatter, showing the sign except for zeroes, with at least 2 fraction digits but no more than 9", () => {
    const itFormatter = createTransferFormatter("it-IT");
    const usFormatter = createTransferFormatter("en-US");

    expect(itFormatter(0)).toBe("0,00");
    expect(usFormatter(0)).toBe("0.00");
    expect(itFormatter(123)).toBe("+123,00");
    expect(usFormatter(123)).toBe("+123.00");
    expect(itFormatter(-123.456_789_456_789)).toBe("-123,456789457");
    expect(usFormatter(-123.456_789_456_789)).toBe("-123.456789457");
  });

  it("should accept BigInts as input", () => {
    const formatter = createTransferFormatter("it-IT");

    expect(formatter(1_234_567n)).toBe("+1.234.567,00");
  });

  it("should throw an error for an invalid locale", () => {
    expect(() => createTransferFormatter("foo-bar")).toThrow();
  });
});
