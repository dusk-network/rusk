import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformBlock, transformTransaction } from "$lib/chain-info";
import { gqlLatestChainInfo } from "$lib/mock-data";
import { appStore } from "$lib/stores";

import HomePage from "../+page.svelte";

describe("home page", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const { chainInfoEntries, fetchInterval, network } = get(appStore);
  const getLatestChainInfoSpy = vi
    .spyOn(duskAPI, "getLatestChainInfo")
    .mockResolvedValue({
      blocks: gqlLatestChainInfo.blocks.map(transformBlock),
      transactions: gqlLatestChainInfo.transactions.map(transformTransaction),
    });

  afterEach(() => {
    cleanup();
    getLatestChainInfoSpy.mockClear();
  });

  afterAll(() => {
    getLatestChainInfoSpy.mockRestore();
    vi.useRealTimers();
  });

  it("should render the home page, start polling for the latest chain info and stop the polling when the component is destroyed", async () => {
    const { container, unmount } = render(HomePage);

    // snapshost in loading state
    expect(container.firstChild).toMatchSnapshot();
    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(1);
    expect(getLatestChainInfoSpy).toHaveBeenNthCalledWith(
      1,
      network,
      chainInfoEntries
    );

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from GraphQL
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(2);
    expect(getLatestChainInfoSpy).toHaveBeenNthCalledWith(
      2,
      network,
      chainInfoEntries
    );

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(3);
    expect(getLatestChainInfoSpy).toHaveBeenNthCalledWith(
      3,
      network,
      chainInfoEntries
    );

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(3);
  });
});
