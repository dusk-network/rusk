import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { apiBlocks } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";
import { BlocksCard } from "..";
import { mapWith, slice } from "lamb";

const transformBlocks = mapWith(transformBlock);
const data = slice(transformBlocks(apiBlocks.data.blocks), 0, 10);

describe("Blocks Card", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));
  const baseProps = {
    blocks: data,
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
    const loading = true;

    const { container, getByRole } = render(BlocksCard, {
      ...baseOptions,
      props: { ...baseProps, loading },
    });

    const button = getByRole("button");

    const showMoreIncrement = 15;

    const clicks = Math.ceil(data.length / showMoreIncrement) - 1;

    Array.from({ length: clicks }).forEach(async () => {
      await fireEvent.click(button);
    });

    expect(button).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });
});
