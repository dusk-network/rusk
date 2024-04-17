import { describe, expect, it } from "vitest";

import { createValueFormatter } from "../";

describe("createValueFormatter", () => {
  it("should format a number correctly", () => {
    const formatter = createValueFormatter("en");
    expect(formatter(9e5)).toBe("900,000");
    expect(formatter(1e6)).toBe("1,000,000");
    expect(formatter(123456.367)).toBe("123,456.367");
  });
});
