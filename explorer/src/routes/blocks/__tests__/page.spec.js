import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformBlock } from "$lib/chain-info";
import { appStore } from "$lib/stores";
import { gqlBlocks } from "$lib/mock-data";
import { changeMediaQueryMatches } from "$lib/dusk/test-helpers";

import Blocks from "../+page.svelte";

describe("Blocks page", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const { blocksListEntries, fetchInterval } = get(appStore);
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
    expect(getBlocksSpy).toHaveBeenNthCalledWith(1, blocksListEntries);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from GraphQL
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(getBlocksSpy).toHaveBeenCalledTimes(2);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(2, blocksListEntries);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getBlocksSpy).toHaveBeenCalledTimes(3);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(3, blocksListEntries);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getBlocksSpy).toHaveBeenCalledTimes(3);
  });

  it("should render the Blocks page with the mobile layout", async () => {
    const { container } = render(Blocks);

    changeMediaQueryMatches("(max-width: 1024px)", true);

    expect(get(appStore).isSmallScreen).toBe(true);

    expect(getBlocksSpy).toHaveBeenCalledTimes(1);
    expect(getBlocksSpy).toHaveBeenNthCalledWith(1, blocksListEntries);

    await vi.advanceTimersByTimeAsync(1);

    expect(container.firstChild).toMatchSnapshot();
  });
});
