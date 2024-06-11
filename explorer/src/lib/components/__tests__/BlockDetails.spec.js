import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { setPathIn } from "lamb";

import { gqlBlock } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";

import { BlockDetails } from "../";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

/**
 * @param {HTMLElement} container
 * @param {"next" | "prev"} which
 * @returns {HTMLAnchorElement?}
 */
const getBlockNavLink = (container, which) =>
  container.querySelector(
    `.block-details__list-anchor:nth-of-type(${which === "prev" ? "1" : "2"})`
  );

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

  it("should disable the previous block link if the prev block hash is empty or if the current height is `0`", async () => {
    const { container, rerender } = render(
      BlockDetails,
      setPathIn(baseProps, "data.header.prevblockhash", "")
    );

    expect(getBlockNavLink(container, "prev")).toHaveAttribute(
      "aria-disabled",
      "true"
    );

    rerender(baseProps);

    expect(getBlockNavLink(container, "prev")).toHaveAttribute(
      "aria-disabled",
      "false"
    );

    rerender(setPathIn(baseProps, "data.header.height", 0));

    expect(getBlockNavLink(container, "prev")).toHaveAttribute(
      "aria-disabled",
      "true"
    );
  });

  it("should disable the next block link if the next block hash is empty", () => {
    const { container } = render(
      BlockDetails,
      setPathIn(baseProps, "data.header.nextblockhash", "")
    );

    expect(getBlockNavLink(container, "next")).toHaveAttribute(
      "aria-disabled",
      "true"
    );
  });
});
