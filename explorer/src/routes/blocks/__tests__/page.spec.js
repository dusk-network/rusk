import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";
import Blocks from "../+page.svelte";

describe("Blocks", () => {
  it("should render the Blocks page", () => {
    const { container } = render(Blocks, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
