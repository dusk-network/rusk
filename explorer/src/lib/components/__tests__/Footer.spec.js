import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Footer } from "../";

describe("Footer", () => {
  afterEach(cleanup);

  it("renders the Footer component", () => {
    const { container } = render(Footer);

    expect(container.firstChild).toMatchSnapshot();
  });
});
