import { describe, expect, it } from "vitest";

import { createCompactFormatter } from "..";

describe("createCompactFormatter", () => {
  it("should format a number correctly", () => {
    const formatter = createCompactFormatter("en");
    expect(formatter(9e5)).toBe("900K");
    expect(formatter(1e6)).toBe("1M");
  });
});
