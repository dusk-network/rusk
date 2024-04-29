import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Navbar } from "../";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Navbar", () => {
  afterEach(cleanup);

  it("renders the Navbar component", () => {
    const { container } = render(Navbar);

    expect(container.firstChild).toMatchSnapshot();
  });
});
