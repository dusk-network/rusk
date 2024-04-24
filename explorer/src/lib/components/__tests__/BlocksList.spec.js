import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { BlocksList } from "..";
import { apiBlock } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

const baseProps = { data: transformBlock(apiBlock.data.blocks[0]) };

describe("Blocks List", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Blocks List component", () => {
    const { container } = render(BlocksList, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });
});
