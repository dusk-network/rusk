import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { BlocksList } from "..";
import { gqlBlock } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";

describe("Blocks List", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const baseProps = { data: transformBlock(gqlBlock.block) };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Blocks List component", () => {
    const { container } = render(BlocksList, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });
});
