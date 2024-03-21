import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";
import Charts from "../+page.svelte";

describe("Charts", () => {
  it("should render the Charts page", () => {
    const { container } = render(Charts, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
