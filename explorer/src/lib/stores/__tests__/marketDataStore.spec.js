import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

import { rejectAfter, resolveAfter } from "$lib/dusk/promise";
import { duskAPI } from "$lib/services";

/**
 * We don't import from "..", because we don't want
 * marketDataStore to be imported and start running
 */
import appStore from "../appStore";

const { fakeMarketDataA, settleTime } = vi.hoisted(() => ({
  fakeMarketDataA: { data: "A" },
  settleTime: 1000,
}));

vi.mock("$lib/services", async (importOriginal) => ({
  .../** @type {import("$lib/services")} */ (await importOriginal()),
  duskAPI: {
    getMarketData: vi.fn(async () => resolveAfter(settleTime, fakeMarketDataA)),
  },
}));

describe("marketDataStore", async () => {
  const { marketDataFetchInterval } = get(appStore);
  const fakeMarketDataB = { data: "B" };

  vi.useFakeTimers();

  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllTimers();
    vi.mocked(duskAPI.getMarketData).mockClear();
  });

  afterAll(() => {
    vi.doUnmock("$lib/services");
    vi.useRealTimers();
  });

  it("should start polling for market data and update the `lastUpdate` property when data changes", async () => {
    const marketDataStore = (await import("../marketDataStore")).default;

    /**
     * This is the result for the second call as the first one
     * starts with the import and isn't resolved yet
     */
    vi.mocked(duskAPI.getMarketData).mockImplementationOnce(() =>
      resolveAfter(settleTime, fakeMarketDataB)
    );

    expect(duskAPI.getMarketData).toHaveBeenCalledTimes(1);
    expect(get(marketDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
      lastUpdate: null,
    });

    await vi.advanceTimersByTimeAsync(settleTime);

    const storeA = {
      data: fakeMarketDataA,
      error: null,
      isLoading: false,
      lastUpdate: new Date(),
    };

    expect(get(marketDataStore)).toStrictEqual(storeA);

    await vi.advanceTimersByTimeAsync(marketDataFetchInterval);

    expect(duskAPI.getMarketData).toHaveBeenCalledTimes(2);
    expect(get(marketDataStore)).toStrictEqual({
      ...storeA,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(settleTime);

    expect(get(marketDataStore)).toStrictEqual({
      data: fakeMarketDataB,
      error: null,
      isLoading: false,
      lastUpdate: new Date(),
    });

    await vi.advanceTimersByTimeAsync(marketDataFetchInterval + settleTime);

    expect(duskAPI.getMarketData).toHaveBeenCalledTimes(3);
    expect(get(marketDataStore)).toStrictEqual({
      ...storeA,
      lastUpdate: new Date(),
    });
  });

  it("should not reset its data and continue polling after an error, without resetting it as well", async () => {
    const marketDataStore = (await import("../marketDataStore")).default;
    const error = new Error("Some error message");

    /**
     * These are the results for the second and third call
     * as the first one starts with the import and isn't resolved yet
     */
    vi.mocked(duskAPI.getMarketData)
      .mockImplementationOnce(() => rejectAfter(settleTime, error))
      .mockImplementationOnce(() => resolveAfter(settleTime, fakeMarketDataB));

    expect(duskAPI.getMarketData).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(settleTime);

    const storeA = {
      data: fakeMarketDataA,
      error: null,
      isLoading: false,
      lastUpdate: new Date(),
    };

    expect(get(marketDataStore)).toStrictEqual(storeA);

    await vi.advanceTimersByTimeAsync(marketDataFetchInterval);

    expect(duskAPI.getMarketData).toHaveBeenCalledTimes(2);
    expect(get(marketDataStore)).toStrictEqual({
      ...storeA,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(settleTime);

    /**
     * The store is loading because after an error the polling
     * restarts immediately and we see only the last store update here.
     */
    expect(get(marketDataStore)).toStrictEqual({
      ...storeA,
      error,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(settleTime);

    expect(get(marketDataStore)).toStrictEqual({
      data: fakeMarketDataB,
      error: null,
      isLoading: false,
      lastUpdate: new Date(),
    });
  });
});
