import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformBlock, transformTransaction } from "$lib/chain-info";
import { apiLatestChainInfo } from "$lib/mock-data";
import { appStore } from "$lib/stores";

import HomePage from "../+page.svelte";

describe("home page", () => {
  vi.useFakeTimers();

  const { fetchInterval, network } = get(appStore);
  const getLatestChainInfoSpy = vi
    .spyOn(duskAPI, "getLatestChainInfo")
    .mockResolvedValue({
      blocks: apiLatestChainInfo.data.blocks.map(transformBlock),
      transactions:
        apiLatestChainInfo.data.transactions.map(transformTransaction),
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

    expect(container.firstChild).toMatchSnapshot();
    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(1);
    expect(getLatestChainInfoSpy).toHaveBeenNthCalledWith(1, network);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(2);
    expect(getLatestChainInfoSpy).toHaveBeenNthCalledWith(2, network);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(3);
    expect(getLatestChainInfoSpy).toHaveBeenNthCalledWith(3, network);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getLatestChainInfoSpy).toHaveBeenCalledTimes(3);
  });
});
