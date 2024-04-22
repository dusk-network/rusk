import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Block from "../+page.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Block", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 15));

  afterEach(() => {
    cleanup();
  });

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the Block Details page", () => {
    const { container } = render(Block, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
