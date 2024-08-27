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

import { createDataStore } from "..";

describe("createDataStore", () => {
  const data1 = { a: 1 };
  const data2 = { a: 2 };
  const data3 = { a: 3 };
  const error = new Error("some error message");
  const args = [1, "a", new Date()];
  const dataRetriever = vi.fn().mockResolvedValue(data1);

  /** @type {DataStore} */
  let dataStore;

  vi.useFakeTimers();

  beforeEach(() => {
    dataStore = createDataStore(dataRetriever);
  });

  afterEach(() => {
    dataRetriever.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should create a readable data store and expose `getData` and `reset` as service method", () => {
    expect(dataRetriever).not.toHaveBeenCalled();
    expect(dataStore).toHaveProperty("getData", expect.any(Function));
    expect(dataStore).toHaveProperty("reset", expect.any(Function));
    expect(dataStore).toHaveProperty("subscribe", expect.any(Function));
    expect(dataStore).not.toHaveProperty("set");
    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });
  });

  it("should set the loading property to `true` when `getData` is called and then fill the data and set loading to `false` if the promise resolves", async () => {
    const expectedState = {
      data: data1,
      error: null,
      isLoading: false,
    };
    const dataPromise = dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);
    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);
  });

  it("should set the loading property to `true` when `getData` is called and then set the error and the loading to `false` if the promise rejects", async () => {
    dataRetriever.mockRejectedValueOnce(error);

    const expectedState = {
      data: null,
      error,
      isLoading: false,
    };
    const dataPromise = dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);
    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);
  });

  it("should ignore the previous call if `getData` is called while the promise is still pending", async () => {
    // test the first promise as success
    dataRetriever
      .mockImplementationOnce(() => resolveAfter(1000, data1))
      .mockResolvedValueOnce(data2);

    /** @type {DataStoreContent} */
    let expectedState = {
      data: data2,
      error: null,
      isLoading: false,
    };

    dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);

    let dataPromise = dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(2);
    expect(dataRetriever).toHaveBeenNthCalledWith(2, ...args);
    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);

    // waiting for the first promise to resolve
    await vi.advanceTimersToNextTimerAsync();

    expect(get(dataStore)).toStrictEqual(expectedState);

    // test the first promise as failure
    dataRetriever
      .mockImplementationOnce(() => rejectAfter(1000, error))
      .mockResolvedValueOnce(data3);

    dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(3);
    expect(dataRetriever).toHaveBeenNthCalledWith(3, ...args);

    expectedState = {
      data: data3,
      error: null,
      isLoading: false,
    };
    dataPromise = dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(4);
    expect(dataRetriever).toHaveBeenNthCalledWith(4, ...args);
    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);

    // waiting for the first promise to resolve
    await vi.advanceTimersToNextTimerAsync();

    expect(get(dataStore)).toStrictEqual(expectedState);

    // test the first promise as success and the second one as failure
    dataRetriever
      .mockImplementationOnce(() => resolveAfter(1000, data1))
      .mockRejectedValueOnce(error);

    dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(5);
    expect(dataRetriever).toHaveBeenNthCalledWith(5, ...args);

    expectedState = {
      data: null,
      error,
      isLoading: false,
    };
    dataPromise = dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(6);
    expect(dataRetriever).toHaveBeenNthCalledWith(6, ...args);
    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);

    // waiting for the first promise to resolve
    await vi.advanceTimersToNextTimerAsync();

    expect(get(dataStore)).toStrictEqual(expectedState);
  });

  it("should clear the error and leave the existing data while the promise is pending and clear the data when it ends with a failure", async () => {
    dataRetriever.mockRejectedValueOnce(error);

    /** @type {DataStoreContent} */
    let expectedState = {
      data: null,
      error,
      isLoading: false,
    };
    let dataPromise = dataStore.getData();

    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);

    expectedState = {
      data: data1,
      error: null,
      isLoading: false,
    };
    dataPromise = dataStore.getData();

    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);

    dataRetriever.mockRejectedValueOnce(error);

    expectedState = {
      data: null,
      error,
      isLoading: false,
    };
    dataPromise = dataStore.getData();

    expect(get(dataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);
  });

  it("should expose a `reset` method to reset the data to its initial state", async () => {
    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });

    await dataStore.getData(...args);

    expect(get(dataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: false,
    });

    dataStore.reset();

    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });

    dataRetriever.mockRejectedValueOnce(error);

    await dataStore.getData(...args);

    expect(get(dataStore)).toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });

    dataStore.reset();

    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });
  });

  it("should ignore the pending promise when `reset` is called and have `getData` return the reset store", async () => {
    const expectedInitialState = {
      data: null,
      error: null,
      isLoading: false,
    };

    await dataStore.getData(...args);

    let dataPromise = dataStore.getData(...args);

    expect(get(dataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: true,
    });

    dataStore.reset();

    expect(await dataPromise).toStrictEqual(expectedInitialState);
    expect(get(dataStore)).toStrictEqual(expectedInitialState);

    await dataStore.getData(...args);

    dataRetriever.mockRejectedValueOnce(error);
    dataPromise = dataStore.getData(...args);

    expect(get(dataStore)).toStrictEqual({
      data: data1,
      error: null,
      isLoading: true,
    });

    dataStore.reset();

    expect(await dataPromise).toStrictEqual(expectedInitialState);
    expect(get(dataStore)).toStrictEqual(expectedInitialState);
  });

  it("should ignore the pending promise when `reset` is called and a `getData` is called immediately afterwards and return the new result", async () => {
    const expectedState = {
      data: data2,
      error: null,
      isLoading: false,
    };

    dataRetriever
      .mockImplementationOnce(() => resolveAfter(1000, data1))
      .mockResolvedValueOnce(data2);

    dataStore.getData(...args);
    dataStore.reset();

    const dataPromise = dataStore.getData(...args);

    expect(await dataPromise).toStrictEqual(expectedState);
    expect(get(dataStore)).toStrictEqual(expectedState);

    // waiting for the first promise to resolve
    await vi.advanceTimersToNextTimerAsync();

    expect(get(dataStore)).toStrictEqual(expectedState);
  });
});
