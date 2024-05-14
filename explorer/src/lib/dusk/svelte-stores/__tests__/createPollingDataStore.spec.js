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

  it("should create a readable data store and expose `reset`, `start` and `stop` as service methods", () => {
    expect(dataRetriever).not.toHaveBeenCalled();
    expect(pollingDataStore).toHaveProperty("reset", expect.any(Function));
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

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });
  });

  it("should be able to restart polling after an error", async () => {
    dataRetriever.mockResolvedValueOnce(data1).mockRejectedValueOnce(error);

    pollingDataStore.start(...args);

    await vi.advanceTimersToNextTimerAsync();

    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(dataRetriever).toHaveBeenCalledTimes(2);

    pollingDataStore.start(...args);

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(dataRetriever).toHaveBeenCalledTimes(13);

    pollingDataStore.stop();
  });

  it("should start a new poll process and stop the previous one when the `start` method is called and a polling is running", async () => {
    dataRetriever.mockImplementation((v) =>
      Promise.resolve(v === 1 ? data1 : data2)
    );

    pollingDataStore.start(1);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenNthCalledWith(1, 1);
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    pollingDataStore.start(2);

    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(dataRetriever).toHaveBeenNthCalledWith(2, 2);

    await vi.advanceTimersByTimeAsync(1);

    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(get(pollingDataStore)).toStrictEqual({
      data: data2,
      error: null,
      isLoading: false,
    });

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(dataRetriever).toHaveBeenCalledTimes(12);

    for (let i = 3; i <= 12; i++) {
      expect(dataRetriever).toHaveBeenNthCalledWith(i, 2);
    }

    pollingDataStore.stop();
    dataRetriever.mockResolvedValue(data1);
  });

  it("should expose a `reset` method that stops the polling and resets the store to its initial state", async () => {
    const expectedInitialState = {
      data: null,
      error: null,
      isLoading: false,
    };

    expect(get(pollingDataStore)).toStrictEqual(expectedInitialState);

    dataRetriever.mockResolvedValueOnce(data1).mockResolvedValueOnce(data2);

    pollingDataStore.start(...args);

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(get(pollingDataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: false,
    });

    expect(dataRetriever).toHaveBeenCalledTimes(1);

    pollingDataStore.reset();

    expect(get(pollingDataStore)).toStrictEqual(expectedInitialState);

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(get(pollingDataStore)).toStrictEqual(expectedInitialState);

    dataRetriever.mockRejectedValueOnce(error).mockResolvedValueOnce(data1);

    pollingDataStore.start(...args);

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(get(pollingDataStore)).toStrictEqual({
      data: data2,
      error: null,
      isLoading: false,
    });

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });

    expect(dataRetriever).toHaveBeenCalledTimes(3);

    pollingDataStore.reset();

    expect(get(pollingDataStore)).toStrictEqual(expectedInitialState);

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(dataRetriever).toHaveBeenCalledTimes(3);
    expect(get(pollingDataStore)).toStrictEqual(expectedInitialState);
  });

  it("should be able to restart a polling after a `reset`", async () => {
    pollingDataStore.start(...args);

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(dataRetriever).toHaveBeenCalledTimes(11);

    pollingDataStore.reset();
    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(get(pollingDataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });

    dataRetriever.mockResolvedValueOnce(data2).mockResolvedValueOnce(data3);
    pollingDataStore.start(...args);

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(get(pollingDataStore)).toStrictEqual({
      data: data2,
      error: null,
      isLoading: false,
    });

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(get(pollingDataStore)).toStrictEqual({
      data: data3,
      error: null,
      isLoading: false,
    });

    pollingDataStore.stop();
  });
});
