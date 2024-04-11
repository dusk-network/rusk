import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Block from "../+page.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Block", () => {
  afterEach(() => {
    cleanup();
  });

  it("should render the Block Details page", () => {
    const { container } = render(Block, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
