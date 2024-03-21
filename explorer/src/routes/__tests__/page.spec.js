import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";
import ChainInfo from "../+page.svelte";

describe("Chain Info", () => {
  it("should render the Chain Info page", () => {
    const { container } = render(ChainInfo, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
