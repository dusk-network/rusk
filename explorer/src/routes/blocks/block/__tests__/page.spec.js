import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { duskAPI } from "$lib/services";
import { transformBlock } from "$lib/chain-info";
import { gqlBlock } from "$lib/mock-data";

import BlockDetails from "../+page.svelte";

describe("Block Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const getBlockSpy = vi
    .spyOn(duskAPI, "getBlock")
    .mockResolvedValue(transformBlock(gqlBlock.block));

  afterEach(() => {
    cleanup();
    getBlockSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getBlockSpy.mockRestore();
  });

  it("should render the Block Details page and query the necessary info", async () => {
    const { container } = render(BlockDetails);

    expect(container.firstChild).toMatchSnapshot();

    expect(getBlockSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();
  });
});
