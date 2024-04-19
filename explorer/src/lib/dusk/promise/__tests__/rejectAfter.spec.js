import { afterAll, describe, expect, it, vi } from "vitest";

import { rejectAfter } from "..";

describe("rejectAfter", () => {
  vi.useFakeTimers();

  afterAll(() => {
    vi.useRealTimers();
  });

  it("build a promise that rejects with the given error after the chosen delay", async () => {
    let result;

    const error = new Error("some error message");
    const delay = 2000;

    rejectAfter(delay, error).catch((r) => {
      result = r;
    });

    await vi.advanceTimersByTimeAsync(delay / 2);

    expect(result).toBeUndefined();

    await vi.advanceTimersByTimeAsync(delay / 2);

    expect(result).toBe(error);
  });
});
