import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
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

vi.mock("svelte/store", async (importOriginal) => {
  /** @type {import("svelte/store")} */
  const original = await importOriginal();

  return {
    ...original,
    get: vi.fn((store) => original.get(store)),
  };
});

vi.mock("$lib/services", async (importOriginal) => ({
  .../** @type {import("$lib/services")} */ (await importOriginal()),
  duskAPI: {
    getMarketData: vi.fn(async () => resolveAfter(settleTime, fakeMarketDataA)),
  },
}));

describe("marketDataStore", async () => {
  const storeKey = "market-data";
  const { marketDataFetchInterval } = get(appStore);
  const fakeMarketDataB = { data: "B" };

  /** @type {MarketDataStore} */
  let marketDataStore;

  vi.useFakeTimers();

  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllTimers();
    vi.mocked(duskAPI.getMarketData).mockClear();

    localStorage.clear();

    marketDataStore = (await import("../marketDataStore")).default;
  });

  afterAll(() => {
    vi.doUnmock("$lib/services");
    vi.doUnmock("svelte/store");
    vi.useRealTimers();
  });

  it("should start polling for market data and update the `lastUpdate` property when data changes", async () => {
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

  it("should not reset its data and stop the polling after an error, without resetting it as well", async () => {
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

    expect(get(marketDataStore)).toStrictEqual({
      ...storeA,
      error,
      isLoading: false,
    });

    await vi.advanceTimersByTimeAsync(marketDataFetchInterval * 10);

    expect(duskAPI.getMarketData).toHaveBeenCalledTimes(2);
  });

  describe("Stale data checks", () => {
    const startingStore = {
      data: null,
      error: null,
      isLoading: false,
      lastUpdate: null,
    };
    const storeWithData = {
      ...startingStore,
      data: fakeMarketDataA,
      lastUpdate: new Date(),
    };

    it("should not consider data as stale if there's no data", () => {
      vi.mocked(get).mockReturnValueOnce(startingStore);

      expect(marketDataStore.isDataStale()).toBe(false);
    });

    it("should not consider data as stale if the store is loading and there is no error, even if the last update exceeds the fetch interval", () => {
      vi.mocked(get)
        .mockReturnValueOnce({ ...startingStore, isLoading: true })
        .mockReturnValueOnce({ ...storeWithData, isLoading: true })
        .mockReturnValueOnce({
          ...storeWithData,
          isLoading: true,
          lastUpdate: new Date(Date.now() - marketDataFetchInterval - 1),
        });

      expect(marketDataStore.isDataStale()).toBe(false);
      expect(marketDataStore.isDataStale()).toBe(false);
      expect(marketDataStore.isDataStale()).toBe(false);
    });

    it("should consider data as stale if there's an error and data, even if the store is loading", () => {
      const storeWithError = {
        ...storeWithData,
        error: new Error("some error"),
      };

      vi.mocked(get)
        .mockReturnValueOnce(storeWithError)
        .mockReturnValueOnce({ ...storeWithError, isLoading: true })
        .mockReturnValueOnce({ ...storeWithError, lastUpdate: null })
        .mockReturnValueOnce({ ...storeWithError, error: null });

      expect(marketDataStore.isDataStale()).toBe(true);
      expect(marketDataStore.isDataStale()).toBe(true);
      expect(marketDataStore.isDataStale()).toBe(false);
      expect(marketDataStore.isDataStale()).toBe(false);
    });

    it("should consider data as stale if the last update exceeds the fetch interval", () => {
      vi.mocked(get)
        .mockReturnValueOnce({
          ...storeWithData,
          lastUpdate: new Date(Date.now() - marketDataFetchInterval - 1),
        })
        .mockReturnValueOnce({
          ...storeWithData,
          lastUpdate: new Date(Date.now() - marketDataFetchInterval),
        });

      expect(marketDataStore.isDataStale()).toBe(true);
      expect(marketDataStore.isDataStale()).toBe(false);
    });
  });

  describe("Handling local storage", () => {
    const consoleErrorSpy = vi.spyOn(console, "error");

    beforeEach(() => {
      vi.resetModules();
      vi.clearAllTimers();
      vi.mocked(duskAPI.getMarketData).mockClear();
    });

    afterEach(() => {
      consoleErrorSpy.mockReset();
    });

    afterAll(() => {
      consoleErrorSpy.mockRestore();
    });

    it("should use data in local storage to initialize the store if present", async () => {
      const storedData = {
        data: "C",
        lastUpdate: new Date(Date.now()),
      };

      localStorage.setItem(storeKey, JSON.stringify(storedData));

      marketDataStore = (await import("../marketDataStore")).default;

      expect(get(marketDataStore)).toStrictEqual({
        error: null,
        isLoading: false,
        ...storedData,
      });
    });

    it("should ignore errors while retrieving local storage data and initialize the store as usual, after logging them in the console", async () => {
      const FakeMarketDataInfo = () => {};

      FakeMarketDataInfo.parse = () => {
        throw new Error("some error");
      };

      vi.doMock("$lib/market-data", async (importOriginal) => ({
        .../** @type {typeof import("$lib/market-data")} */ (
          await importOriginal()
        ),
        MarketDataInfo: FakeMarketDataInfo,
      }));
      localStorage.setItem(storeKey, "{}");

      // we don't want to see our fake error in the console
      consoleErrorSpy.mockImplementationOnce(() => {});

      marketDataStore = (await import("../marketDataStore")).default;

      expect(consoleErrorSpy).toHaveBeenCalledTimes(1);
      expect(get(marketDataStore)).toStrictEqual({
        data: null,
        error: null,
        isLoading: true,
        lastUpdate: null,
      });

      vi.doUnmock("$lib/market-data");
    });

    it("should start the polling as usual if there's data stored, but it's stale", async () => {
      localStorage.setItem(
        storeKey,
        JSON.stringify({
          data: "D",
          lastUpdate: new Date(Date.now() - marketDataFetchInterval - 1),
        })
      );

      marketDataStore = (await import("../marketDataStore")).default;

      expect(duskAPI.getMarketData).toHaveBeenCalledTimes(1);

      await vi.advanceTimersByTimeAsync(settleTime + marketDataFetchInterval);

      expect(duskAPI.getMarketData).toHaveBeenCalledTimes(2);
    });

    it("should delay the polling if there's data stored and it's not stale", async () => {
      const offset = Math.floor(marketDataFetchInterval / 2);
      const expectedDelay = marketDataFetchInterval - offset;

      localStorage.setItem(
        storeKey,
        JSON.stringify({
          data: "D",
          lastUpdate: new Date(Date.now() - marketDataFetchInterval + offset),
        })
      );

      marketDataStore = (await import("../marketDataStore")).default;

      expect(duskAPI.getMarketData).not.toHaveBeenCalled();

      await vi.advanceTimersByTimeAsync(expectedDelay - 1);

      expect(duskAPI.getMarketData).not.toHaveBeenCalled();

      await vi.advanceTimersByTimeAsync(1);

      expect(duskAPI.getMarketData).toHaveBeenCalledTimes(1);

      await vi.advanceTimersByTimeAsync(marketDataFetchInterval + settleTime);

      expect(duskAPI.getMarketData).toHaveBeenCalledTimes(2);
    });

    it("should save the received data in local storage if the request has new data", async () => {
      marketDataStore = (await import("../marketDataStore")).default;

      await vi.advanceTimersByTimeAsync(settleTime);

      const expectedStorage = {
        data: fakeMarketDataA,
        lastUpdate: new Date(),
      };
      const expectedStore = {
        ...expectedStorage,
        error: null,
        isLoading: false,
      };

      expect(duskAPI.getMarketData).toHaveBeenCalledTimes(1);
      expect(get(marketDataStore)).toStrictEqual(expectedStore);
      expect(localStorage.getItem(storeKey)).toStrictEqual(
        JSON.stringify(expectedStorage)
      );
    });

    it("should leave the local storage as it is if the market data request ends with an error", async () => {
      const error = new Error("some error");

      vi.mocked(duskAPI.getMarketData).mockImplementationOnce(() =>
        rejectAfter(settleTime, error)
      );

      marketDataStore = (await import("../marketDataStore")).default;

      await vi.advanceTimersByTimeAsync(settleTime);

      expect(get(marketDataStore)).toStrictEqual({
        data: null,
        error,
        isLoading: false,
        lastUpdate: null,
      });

      expect(localStorage.getItem(storeKey)).toBeNull();
    });

    it("should ignore errors while writing to the storage and continue polling as usual", async () => {
      const setDataSpy = vi
        .spyOn(Storage.prototype, "setItem")
        .mockImplementation(() => {
          throw new Error("some error");
        });

      // we don't want to see our fake error in the console
      consoleErrorSpy.mockImplementationOnce(() => {});

      marketDataStore = (await import("../marketDataStore")).default;

      await vi.advanceTimersByTimeAsync(settleTime);

      expect(setDataSpy).not.toHaveBeenCalled();
      expect(get(marketDataStore)).toStrictEqual({
        data: fakeMarketDataA,
        error: null,
        isLoading: false,
        lastUpdate: new Date(),
      });

      expect(consoleErrorSpy).toHaveBeenCalledTimes(1);

      setDataSpy.mockRestore();
    });
  });
});
