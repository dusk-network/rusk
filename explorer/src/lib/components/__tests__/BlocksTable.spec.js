import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { apiBlocks } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";
import { BlocksTable } from "..";
import { mapWith, slice } from "lamb";

const transformBlocks = mapWith(transformBlock);
const data = slice(transformBlocks(apiBlocks.data.blocks), 0, 10);

describe("Blocks Table", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));
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
