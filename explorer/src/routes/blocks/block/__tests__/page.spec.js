import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformBlock } from "$lib/chain-info";
import { gqlBlock, gqlBlockDetails } from "$lib/mock-data";
import { changeMediaQueryMatches } from "$lib/dusk/test-helpers";

import BlockDetails from "../+page.svelte";

describe("Block Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const getBlockSpy = vi
    .spyOn(duskAPI, "getBlock")
    .mockResolvedValue(transformBlock(gqlBlock.block));

  const getBlockDetailsSpy = vi
    .spyOn(duskAPI, "getBlockDetails")
    .mockResolvedValue(gqlBlockDetails.block.header.json);

  afterEach(() => {
    cleanup();
    getBlockSpy.mockClear();
    getBlockDetailsSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getBlockSpy.mockRestore();
    getBlockDetailsSpy.mockRestore();
  });

  it("should render the Block Details page and query the necessary info", async () => {
    const { container } = render(BlockDetails);

    expect(container.firstChild).toMatchSnapshot();

    expect(getBlockSpy).toHaveBeenCalledTimes(1);
    expect(getBlockDetailsSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the Transaction section of the Block Details page with the mobile layout", async () => {
    const { appStore } = await import("$lib/stores");
    const { container } = render(BlockDetails);

    changeMediaQueryMatches("(max-width: 1024px)", true);

    expect(get(appStore).isSmallScreen).toBe(true);

    expect(getBlockSpy).toHaveBeenCalledTimes(1);
    expect(getBlockDetailsSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);

    expect(container.firstChild).toMatchSnapshot();
  });
});
