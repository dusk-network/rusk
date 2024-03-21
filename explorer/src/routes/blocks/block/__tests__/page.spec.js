import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";
import Block from "../+page.svelte";

describe("Block", () => {
  it("should render the Block page", () => {
    const { container } = render(Block, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
