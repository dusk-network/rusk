import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Navbar } from "../";

describe("Navbar", () => {
  afterEach(cleanup);

  it("renders the Navbar component", () => {
    const { container } = render(Navbar);

    expect(container.firstChild).toMatchSnapshot();
  });
});
