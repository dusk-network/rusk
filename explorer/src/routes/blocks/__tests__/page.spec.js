import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformBlock } from "$lib/chain-info";
import { appStore } from "$lib/stores";
import { apiBlocks } from "$lib/mock-data";

import Blocks from "../+page.svelte";

describe("Blocks page", () => {
  vi.useFakeTimers();

  const { fetchInterval, network } = get(appStore);
  const getBlocksSpy = vi
    .spyOn(duskAPI, "getBlocks")
    .mockResolvedValue(apiBlocks.data.blocks.map(transformBlock));

  afterEach(() => {
    cleanup();
    getBlocksSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getBlocksSpy.mockRestore();
  });

  it("should render the Blocks page, start polling for blocks and stop the polling when unmounted", async () => {
    const { container, unmount } = render(Blocks);

    expect(container.firstChild).toMatchSnapshot();
    expect(getBlocksSpy).toHaveBeenCalledTimes(1);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(1, network);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(getBlocksSpy).toHaveBeenCalledTimes(2);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(2, network);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getBlocksSpy).toHaveBeenCalledTimes(3);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(3, network);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getBlocksSpy).toHaveBeenCalledTimes(3);
  });
});
