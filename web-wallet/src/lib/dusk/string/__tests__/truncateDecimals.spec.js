/**
 * Tests for the `truncateDecimals` function using Vitest.
 *
 * This file defines a local copy of the `truncateDecimals` implementation
 * based on the problem statement and exercises a variety of inputs to
 * ensure the function behaves correctly. The tests cover numbers with
 * both dot and comma separators, varying lengths of decimal portions,
 * cases with no decimals at all, and negative values. Edge cases such
 * as numbers ending with a separator are also included.
 */

import { describe, expect, it } from "vitest";
import { truncateDecimals } from "..";

const DUSK_DECIMALS = 9;

describe("truncateDecimals", () => {
  it("returns the original string when there is no decimal part", () => {
    expect(truncateDecimals("123", DUSK_DECIMALS)).toBe("123");
    // A trailing separator with no decimals should also return the original string
    expect(truncateDecimals("123.", DUSK_DECIMALS)).toBe("123.");
  });

  it("truncates decimals using a dot as the separator", () => {
    expect(truncateDecimals("123.4567890123", DUSK_DECIMALS)).toBe(
      "123.456789012"
    );
    // Decimal portion shorter than the limit should remain unchanged
    expect(truncateDecimals("123.4567", DUSK_DECIMALS)).toBe("123.4567");
  });

  it("truncates decimals using a comma as the separator", () => {
    expect(truncateDecimals("123,4567890123", DUSK_DECIMALS)).toBe(
      "123,456789012"
    );
    // Decimal portion shorter than the limit should remain unchanged
    expect(truncateDecimals("123,4567", DUSK_DECIMALS)).toBe("123,4567");
  });

  it("handles negative numbers correctly", () => {
    expect(truncateDecimals("-123.4567890123", DUSK_DECIMALS)).toBe(
      "-123.456789012"
    );
    expect(truncateDecimals("-123,4567890123", DUSK_DECIMALS)).toBe(
      "-123,456789012"
    );
  });

  it("uses the dot separator when both separators are present", () => {
    // The function chooses '.' if present, treating ',' as part of the integer
    expect(truncateDecimals("1,234.56789", DUSK_DECIMALS)).toBe("1,234.56789");
  });
});
