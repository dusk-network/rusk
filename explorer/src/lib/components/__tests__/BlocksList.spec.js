import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { setPathIn } from "lamb";

import { gqlBlock } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";

import { BlocksList } from "..";

/** @param {HTMLElement} container */
function getTimeElement(container) {
  return /** @type {HTMLTimeElement} */ (container.querySelector("time"));
}

describe("Blocks List", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const block = transformBlock(gqlBlock.block);

  /** @type {import("svelte").ComponentProps<BlocksList>} */
  const baseProps = { data: block };

  const timeRefreshInterval = 1000;

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Blocks List component", () => {
    const { container } = render(BlocksList, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should auto-refresh relative times when the related prop is set to true", async () => {
    const props = {
      ...baseProps,
      data: setPathIn(block, "header.date", new Date()),
    };
    const { container, rerender } = render(BlocksList, props);
    const timeElement = getTimeElement(container);

    expect(timeElement.innerHTML).toMatchInlineSnapshot(`"now"`);

    await vi.advanceTimersByTimeAsync(timeRefreshInterval * 3);

    expect(timeElement.innerHTML).toMatchInlineSnapshot(`"now"`);

    await rerender({ ...props, autoRefreshTime: true });

    expect(timeElement.innerHTML).toMatchInlineSnapshot(`"3 seconds ago"`);

    await vi.advanceTimersByTimeAsync(timeRefreshInterval);

    expect(timeElement.innerHTML).toMatchInlineSnapshot(`"4 seconds ago"`);

    await vi.advanceTimersByTimeAsync(timeRefreshInterval);

    expect(timeElement.innerHTML).toMatchInlineSnapshot(`"5 seconds ago"`);
  });
});
