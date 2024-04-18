import { describe, expect, it } from "vitest";

import { unixTsToDate } from "..";

describe("unixTsToDate", () => {
  it("should transform a Unix timestamp in a JS Date", () => {
    const result = unixTsToDate(1713251109);

    expect(result).toBeInstanceOf(Date);
    expect(result.toISOString()).toBe("2024-04-16T07:05:09.000Z");
  });
});
