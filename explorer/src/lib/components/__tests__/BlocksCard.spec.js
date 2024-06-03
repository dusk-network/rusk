import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { compose, mapWith, take } from "lamb";

import { gqlBlocks } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";

import { BlocksCard } from "..";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Blocks Card", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const getTenBlocks = compose(mapWith(transformBlock), take(10));
  const data = getTenBlocks(gqlBlocks.blocks);

  const baseProps = {
    blocks: null,
    error: null,
    loading: false,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the `BlocksCard` component", () => {
    const { container } = render(BlocksCard, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button is the card is in the loading state", () => {
    const loading = true;

    const { container, getByRole } = render(BlocksCard, {
      ...baseOptions,
      props: { ...baseProps, loading },
    });

    expect(getByRole("button")).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button if there is no more data to display", async () => {
    const loading = false;
    const blocks = data;

    const { container, getByRole } = render(BlocksCard, {
      ...baseOptions,
      props: { ...baseProps, blocks, loading },
    });

    expect(getByRole("button")).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });
});
