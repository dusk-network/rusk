import { afterAll, beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { tick } from "svelte";

import { createPollingDataStore } from "..";

describe("createPollingDataStore", () => {
  const data1 = { a: 1 };
  const data2 = { a: 2 };
  const data3 = { a: 3 };
  const error = new Error("some error message");
  const args = [1, "a", new Date()];
  const dataRetriever = vi.fn().mockResolvedValue(data1);
  const fetchInterval = 1000;

  /** @type {PollingDataStore} */
  let pollingDataStore;

  vi.useFakeTimers();

  beforeEach(() => {
    dataRetriever.mockClear();
    pollingDataStore = createPollingDataStore(dataRetriever, fetchInterval);
  });

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should create a readable data store and expose `start` and `stop` as service methods", () => {
    expect(dataRetriever).not.toHaveBeenCalled();
    expect(pollingDataStore).toHaveProperty("start", expect.any(Function));
    expect(pollingDataStore).toHaveProperty("stop", expect.any(Function));
    expect(pollingDataStore).toHaveProperty("subscribe", expect.any(Function));
    expect(pollingDataStore).not.toHaveProperty("set");
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });
  });

  it("should start polling data with the desired interval when the `start` method is called and stop when the `stop` method is called", async () => {
    dataRetriever
      .mockResolvedValueOnce(data1)
      .mockResolvedValueOnce(data2)
      .mockResolvedValueOnce(data3);

    pollingDataStore.start(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(1);

    // no other call happened yet
    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: false,
    });

    vi.advanceTimersByTime(fetchInterval - 1);
    await tick();

    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(dataRetriever).toHaveBeenNthCalledWith(2, ...args);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(1);

    // no other call happened yet
    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data2,
      error: null,
      isLoading: false,
    });

    vi.advanceTimersByTime(fetchInterval - 1);
    await tick();

    expect(dataRetriever).toHaveBeenCalledTimes(3);
    expect(dataRetriever).toHaveBeenNthCalledWith(3, ...args);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data2,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(1);

    // no other call happened yet
    expect(dataRetriever).toHaveBeenCalledTimes(3);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data3,
      error: null,
      isLoading: false,
    });

    pollingDataStore.stop();

    vi.advanceTimersByTime(fetchInterval - 1);
    await tick();

    expect(dataRetriever).toHaveBeenCalledTimes(3);
  });

  it("should stop the polling if an error occurs during the calls", async () => {
    dataRetriever.mockResolvedValueOnce(data1).mockRejectedValueOnce(error);

    pollingDataStore.start(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(1);

    // no other call happened yet
    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: false,
    });

    vi.advanceTimersByTime(fetchInterval - 1);
    await tick();

    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(dataRetriever).toHaveBeenNthCalledWith(2, ...args);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(1);

    // no other call happened yet
    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });

    vi.advanceTimersByTime(fetchInterval - 1);
    await tick();

    expect(dataRetriever).toHaveBeenCalledTimes(2);
  });

  it("should not make a new call when the `start` method is invoked and the polling is already running", async () => {
    pollingDataStore.start(...args);
    pollingDataStore.start(...args);

    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersByTimeAsync(1);

    expect(get(pollingDataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: false,
    });

    pollingDataStore.start(...args);
    pollingDataStore.start(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);

    pollingDataStore.stop();
  });
});
