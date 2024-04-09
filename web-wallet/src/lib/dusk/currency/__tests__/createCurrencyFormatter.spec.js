import { describe, expect, it } from "vitest";

import { createCurrencyFormatter } from "..";

describe("createCurrencyFormatter", () => {
  it("should create a currency formatter using the given settings, but ignore the digits for FIAT currencies", () => {
    const itFormatter = createCurrencyFormatter("it-IT", "EUR", 9);
    const usFormatter = createCurrencyFormatter("en-US", "USD", 9);

    /**
     * A non-breaking space is used here (code point 0x00A0)
     * @see https://shapecatcher.com/unicode/info/160
     */
    expect(itFormatter(1_234_567.899)).toBe("1.234.567,90 €");
    expect(usFormatter(1_234_567.899)).toBe("$1,234,567.90");
  });

  it('should accept "DUSK" as a currency, using the desired digits', () => {
    const duskFormatter = createCurrencyFormatter("it-IT", "DUSK", 5);

    expect(duskFormatter(1_234_567.899_788_677)).toBe("1.234.567,89979");
  });

  it("should accept BigInts as input", () => {
    const duskFormatter = createCurrencyFormatter("it-IT", "DUSK", 5);

    expect(duskFormatter(1_234_567n)).toBe("1.234.567,00000");
  });

  it("should throw an error for an invalid locale", () => {
    expect(() => createCurrencyFormatter("foo-bar", "DUSK", 5)).toThrow();
  });

  it("should throw an error for an invalid currency", () => {
    expect(() => createCurrencyFormatter("it-IT", "foobarbaz", 5)).toThrow();
  });
});
