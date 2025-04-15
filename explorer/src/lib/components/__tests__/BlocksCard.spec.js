import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { compose, mapWith, take } from "lamb";

import { gqlBlocks } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";
import { BlocksCard } from "..";

describe("Blocks Card", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const getTenBlocks = compose(mapWith(transformBlock), take(10));
  const data = getTenBlocks(gqlBlocks.blocks);

  const baseProps = {
    blocks: null,
    error: null,
    isSmallScreen: false,
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

    expect(getByRole("button", { name: "Show More" })).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button if there is no more data to display", async () => {
    const loading = false;
    const blocks = data;

    const { container, getByRole } = render(BlocksCard, {
      ...baseOptions,
      props: { ...baseProps, blocks, loading },
    });

    expect(getByRole("button", { name: "Show More" })).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should hide the `Show More` button if an error has occurred", async () => {
    const props = { ...baseProps, error: new Error("error") };

    const { container } = render(BlocksCard, {
      ...baseOptions,
      props: { ...props, error: new Error("error") },
    });

    expect(container.firstChild).toMatchSnapshot();
  });
});
