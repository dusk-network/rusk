import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformBlock } from "$lib/chain-info";
import { appStore } from "$lib/stores";
import { gqlBlocks } from "$lib/mock-data";

import Blocks from "../+page.svelte";

describe("Blocks page", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const { blocksListEntries, fetchInterval, network } = get(appStore);
  const getBlocksSpy = vi
    .spyOn(duskAPI, "getBlocks")
    .mockResolvedValue(gqlBlocks.blocks.map(transformBlock));

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

    // snapshost in loading state
    expect(container.firstChild).toMatchSnapshot();
    expect(getBlocksSpy).toHaveBeenCalledTimes(1);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(1, network, blocksListEntries);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from GraphQL
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(getBlocksSpy).toHaveBeenCalledTimes(2);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(2, network, blocksListEntries);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getBlocksSpy).toHaveBeenCalledTimes(3);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(3, network, blocksListEntries);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getBlocksSpy).toHaveBeenCalledTimes(3);
  });
});
