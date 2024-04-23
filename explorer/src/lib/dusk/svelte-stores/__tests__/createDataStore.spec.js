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

import { createDataStore } from "..";

describe("createDataStore", () => {
  const data = {};
  const error = new Error("some error message");
  const args = [1, "a", new Date()];
  const dataRetriever = vi.fn().mockResolvedValue(data);

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

  it("should create a readable data store and expose a `getData` service method", () => {
    expect(dataRetriever).not.toHaveBeenCalled();
    expect(dataStore).toHaveProperty("getData", expect.any(Function));
    expect(dataStore).toHaveProperty("subscribe", expect.any(Function));
    expect(dataStore).not.toHaveProperty("set");
    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: false,
    });
  });

  it("should set the loading property to `true` when `getData` is called and then fill the data and set loading to `false` if the promise resolves", async () => {
    const dataPromise = dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);
    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(dataPromise).resolves.toStrictEqual({
      data,
      error: null,
      isLoading: false,
    });
    expect(dataPromise).resolves.toStrictEqual(get(dataStore));
  });

  it("should set the loading property to `true` when `getData` is called and then set the error and the loading to `false` if the promise rejects", async () => {
    dataRetriever.mockRejectedValueOnce(error);

    const dataPromise = dataStore.getData(...args);

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(...args);
    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(dataPromise).resolves.toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });
    expect(dataPromise).resolves.toStrictEqual(get(dataStore));
  });

  it("should not call the data retriever when `getData` is called and the promise is still pending", async () => {
    const dataPromise = dataStore.getData(1);

    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    const dataPromise2 = dataStore.getData(2);
    const dataPromise3 = dataStore.getData(3);

    expect(dataPromise2).toBe(dataPromise);
    expect(dataPromise3).toBe(dataPromise);

    await vi.advanceTimersToNextTimerAsync();

    expect(dataRetriever).toHaveBeenCalledTimes(1);
    expect(dataRetriever).toHaveBeenCalledWith(1);
    expect(dataPromise).resolves.toStrictEqual({
      data,
      error: null,
      isLoading: false,
    });
    expect(dataPromise).resolves.toStrictEqual(get(dataStore));
  });

  it("should clear the error and leave the existing data while the promise is pending and clear the data when it ends with a failure", async () => {
    dataRetriever.mockRejectedValueOnce(error);

    let dataPromise = dataStore.getData();

    await vi.advanceTimersToNextTimerAsync();

    expect(dataPromise).resolves.toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });
    expect(dataPromise).resolves.toStrictEqual(get(dataStore));

    dataPromise = dataStore.getData();

    expect(get(dataStore)).toStrictEqual({
      data: null,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(dataPromise).resolves.toStrictEqual({
      data,
      error: null,
      isLoading: false,
    });
    expect(dataPromise).resolves.toStrictEqual(get(dataStore));

    dataRetriever.mockRejectedValueOnce(error);

    dataPromise = dataStore.getData();

    expect(get(dataStore)).toStrictEqual({
      data,
      error: null,
      isLoading: true,
    });

    await vi.advanceTimersToNextTimerAsync();

    expect(dataPromise).resolves.toStrictEqual({
      data: null,
      error,
      isLoading: false,
    });
    expect(dataPromise).resolves.toStrictEqual(get(dataStore));
  });
});
