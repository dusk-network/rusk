import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { gqlBlock } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";
import { BlockDetails } from "../";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Block Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const baseProps = {
    data: transformBlock(gqlBlock.block),
    error: null,
    loading: false,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Block Details component", () => {
    const { container } = render(BlockDetails, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });
});
