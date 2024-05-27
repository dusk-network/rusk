import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformBlock } from "$lib/chain-info";
import { apiBlock } from "$lib/mock-data";
import { appStore } from "$lib/stores";

import BlockDetails from "../+page.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Block Details", () => {
  vi.useFakeTimers();

  const { fetchInterval } = get(appStore);
  const getBlockSpy = vi
    .spyOn(duskAPI, "getBlock")
    .mockResolvedValue(transformBlock(apiBlock.data.blocks[0]));

  afterEach(() => {
    cleanup();
    getBlockSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getBlockSpy.mockRestore();
  });

  it("should render the Block Details page, start polling the block data and stop the polling when unmounted", async () => {
    const { container, unmount } = render(BlockDetails);

    expect(container.firstChild).toMatchSnapshot();

    expect(getBlockSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getBlockSpy).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getBlockSpy).toHaveBeenCalledTimes(3);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getBlockSpy).toHaveBeenCalledTimes(3);
  });
});
