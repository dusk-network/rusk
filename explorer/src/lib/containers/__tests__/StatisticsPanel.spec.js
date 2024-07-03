import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { resolveAfter } from "$lib/dusk/promise";
import { duskAPI } from "$lib/services";
import { appStore } from "$lib/stores";

import { StatisticsPanel } from "..";

const marketDataSettleTime = vi.hoisted(() => {
  vi.useFakeTimers();

  return 100;
});
vi.mock("$lib/services", async (importOriginal) => {
  /** @type {import("$lib/services")} */
  const original = await importOriginal();
  const { apiMarketData, apiNodeLocations, apiStats } = await import(
    "$lib/mock-data"
  );
  const { current_price: currentPrice, market_cap: marketCap } =
    apiMarketData.market_data;

  return {
    ...original,
    duskAPI: {
      ...original.duskAPI,
      getMarketData: () =>
        resolveAfter(marketDataSettleTime, { currentPrice, marketCap }),
      getNodeLocations: vi.fn().mockResolvedValue(apiNodeLocations.data),
      getStats: vi.fn().mockResolvedValue(apiStats),
    },
  };
});

describe("StatisticsPanel", () => {
  const { fetchInterval, network } = get(appStore);

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
    vi.doUnmock("$lib/services");
  });

  it("should render the StatisticsPanel, query for the necessary info, start polling for stats and stop the polling when unmounted", async () => {
    const { container, unmount } = render(StatisticsPanel);

    expect(container.firstChild).toMatchSnapshot();
    expect(duskAPI.getNodeLocations).toHaveBeenCalledTimes(1);
    expect(duskAPI.getNodeLocations).toHaveBeenNthCalledWith(1, network);
    expect(duskAPI.getStats).toHaveBeenCalledTimes(1);
    expect(duskAPI.getStats).toHaveBeenNthCalledWith(1, network);

    await vi.advanceTimersByTimeAsync(marketDataSettleTime);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - marketDataSettleTime);

    expect(duskAPI.getStats).toHaveBeenCalledTimes(2);
    expect(duskAPI.getStats).toHaveBeenNthCalledWith(2, network);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(duskAPI.getStats).toHaveBeenCalledTimes(3);
    expect(duskAPI.getStats).toHaveBeenNthCalledWith(3, network);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(duskAPI.getStats).toHaveBeenCalledTimes(3);
    expect(duskAPI.getNodeLocations).toHaveBeenCalledTimes(1);
  });
});
