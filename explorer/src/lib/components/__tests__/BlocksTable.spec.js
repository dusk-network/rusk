import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { mapWith, slice } from "lamb";

import { gqlBlocks } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";

import { BlocksTable } from "..";

describe("Blocks Table", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const transformBlocks = mapWith(transformBlock);
  const data = slice(transformBlocks(gqlBlocks.blocks), 0, 10);

  const baseProps = {
    data: data,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the `BlocksTable` component", () => {
    const { container } = render(BlocksTable, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });
});
