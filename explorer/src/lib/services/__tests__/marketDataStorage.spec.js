import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent } from "@testing-library/svelte";

import { marketDataStorage } from "..";

describe("marketDataStorage", () => {
  const marketData = { data: { a: 1 }, lastUpdate: new Date() };

  beforeEach(() => {
    localStorage.setItem("market-data", JSON.stringify(marketData));
  });

  it("should expose a method to clear the market data storage", async () => {
    localStorage.setItem("some-key", "some value");

    await expect(marketDataStorage.clear()).resolves.toBe(undefined);

    expect(localStorage.getItem("market-data")).toBeNull();
    expect(localStorage.getItem("some-key")).toBe("some value");
  });

  it("should expose a method to retrieve market data from the storage", async () => {
    await expect(marketDataStorage.get()).resolves.toStrictEqual(marketData);
  });

  it("should expose a method to set the data in the storage", async () => {
    const newData = { data: { b: 2 }, lastUpdate: new Date() };

    // @ts-expect-error
    await expect(marketDataStorage.set(newData)).resolves.toBe(undefined);
    await expect(marketDataStorage.get()).resolves.toStrictEqual(newData);

    expect(localStorage.getItem("market-data")).toBe(JSON.stringify(newData));
  });

  it("should expose a method that allows to set a listener for storage events and returns a function to remove the listener", async () => {
    const eventA = new StorageEvent("storage", { key: "market-data" });
    const eventB = new StorageEvent("storage", { key: "some-other-key" });
    const listener = vi.fn();
    const removeListener = marketDataStorage.onChange(listener);

    await fireEvent(window, eventA);

    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith(eventA);

    await fireEvent(window, eventB);

    expect(listener).toHaveBeenCalledTimes(1);

    removeListener();

    await fireEvent(window, eventA);

    expect(listener).toHaveBeenCalledTimes(1);
  });
});
