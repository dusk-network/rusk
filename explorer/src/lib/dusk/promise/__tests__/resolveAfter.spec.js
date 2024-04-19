import { afterAll, describe, expect, it, vi } from "vitest";

import { resolveAfter } from "..";

describe("resolveAfter", () => {
  vi.useFakeTimers();

  afterAll(() => {
    vi.useRealTimers();
  });

  it("build a promise that resolves with the given value after the chosen delay", async () => {
    let result;

    const value = {};
    const delay = 2000;

    resolveAfter(delay, value).then((r) => {
      result = r;
    });

    await vi.advanceTimersByTimeAsync(delay / 2);

    expect(result).toBeUndefined();

    await vi.advanceTimersByTimeAsync(delay / 2);

    expect(result).toBe(value);
  });
});
