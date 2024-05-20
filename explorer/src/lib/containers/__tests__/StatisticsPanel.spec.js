import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { appStore } from "$lib/stores";
import { apiMarketData, apiNodeLocations, apiStats } from "$lib/mock-data";

import { StatisticsPanel } from "..";

describe("StatisticsPanel", () => {
  vi.useFakeTimers();

  const { fetchInterval, network } = get(appStore);
  const getMarketDataSpy = vi
    .spyOn(duskAPI, "getMarketData")
    .mockResolvedValue({
      currentPrice: apiMarketData.market_data.current_price,
      marketCap: apiMarketData.market_data.market_cap,
    });
  const getNodeLocationsSpy = vi
    .spyOn(duskAPI, "getNodeLocations")
    .mockResolvedValue(apiNodeLocations.data);
  const getStatsSpy = vi.spyOn(duskAPI, "getStats").mockResolvedValue(apiStats);

  afterEach(() => {
    cleanup();
    getMarketDataSpy.mockClear();
    getNodeLocationsSpy.mockClear();
    getStatsSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getMarketDataSpy.mockRestore();
    getNodeLocationsSpy.mockRestore();
    getStatsSpy.mockRestore();
  });

  it("should render the StatisticsPanel, query for the necessary info, start polling for stats and stop the polling when unmounted", async () => {
    const { container, unmount } = render(StatisticsPanel);

    expect(container.firstChild).toMatchSnapshot();
    expect(getMarketDataSpy).toHaveBeenCalledTimes(1);
    expect(getMarketDataSpy).toHaveBeenNthCalledWith(1);
    expect(getNodeLocationsSpy).toHaveBeenCalledTimes(1);
    expect(getNodeLocationsSpy).toHaveBeenNthCalledWith(1, network);
    expect(getStatsSpy).toHaveBeenCalledTimes(1);
    expect(getStatsSpy).toHaveBeenNthCalledWith(1, network);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(getStatsSpy).toHaveBeenCalledTimes(2);
    expect(getStatsSpy).toHaveBeenNthCalledWith(2, network);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getStatsSpy).toHaveBeenCalledTimes(3);
    expect(getStatsSpy).toHaveBeenNthCalledWith(3, network);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getStatsSpy).toHaveBeenCalledTimes(3);
    expect(getNodeLocationsSpy).toHaveBeenCalledTimes(1);
    expect(getMarketDataSpy).toHaveBeenCalledTimes(1);
  });
});
