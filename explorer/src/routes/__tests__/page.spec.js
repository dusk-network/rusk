import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { resolveAfter } from "$lib/dusk/promise";
import { duskAPI } from "$lib/services";

import { appStore } from "$lib/stores";

import HomePage from "../+page.svelte";

const marketDataSettleTime = vi.hoisted(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  return 100;
});
vi.mock("$lib/services", async (importOriginal) => {
  /** @type {import("$lib/services")} */
  const original = await importOriginal();
  const { transformBlock, transformTransaction } = await import(
    "$lib/chain-info"
  );
  const { apiMarketData, gqlLatestChainInfo, nodeLocationsCount } =
    await import("$lib/mock-data");
  const { current_price: currentPrice, market_cap: marketCap } =
    apiMarketData.market_data;

  return {
    ...original,
    duskAPI: {
      ...original.duskAPI,
      getLatestChainInfo: vi.fn().mockResolvedValue({
        blocks: gqlLatestChainInfo.blocks.map(transformBlock),
        transactions: gqlLatestChainInfo.transactions.map(transformTransaction),
      }),
      getMarketData: () =>
        resolveAfter(marketDataSettleTime, { currentPrice, marketCap }),
      getNodeLocations: vi.fn().mockResolvedValue(nodeLocationsCount),
    },
  };
});

describe("home page", () => {
  const { chainInfoEntries, fetchInterval } = get(appStore);

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  afterAll(() => {
    vi.useRealTimers();
    vi.doUnmock("$lib/services");
  });

  it("should render the home page, start polling for the latest chain info and stop the polling when the component is destroyed", async () => {
    const { container, unmount } = render(HomePage);

    // snapshost in loading state
    expect(container.firstChild).toMatchSnapshot();
    expect(duskAPI.getLatestChainInfo).toHaveBeenCalledTimes(1);
    expect(duskAPI.getLatestChainInfo).toHaveBeenNthCalledWith(
      1,
      chainInfoEntries
    );

    await vi.advanceTimersByTimeAsync(marketDataSettleTime);

    // snapshot with received data from GraphQL
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - marketDataSettleTime);

    expect(duskAPI.getLatestChainInfo).toHaveBeenCalledTimes(2);
    expect(duskAPI.getLatestChainInfo).toHaveBeenNthCalledWith(
      2,
      chainInfoEntries
    );

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(duskAPI.getLatestChainInfo).toHaveBeenCalledTimes(3);
    expect(duskAPI.getLatestChainInfo).toHaveBeenNthCalledWith(
      3,
      chainInfoEntries
    );

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(duskAPI.getLatestChainInfo).toHaveBeenCalledTimes(3);
  });
});
