import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent } from "@testing-library/svelte";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

import MainLayout from "../+layout.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Main layout", () => {
  const baseOptions = { props: {}, target: document.body };

  afterEach(cleanup);

  it("should render the app's main layout", () => {
    const { container } = renderWithSimpleContent(MainLayout, baseOptions);

    expect(container).toMatchSnapshot();
  });

  it("should change the overflow of document's body when the navbar toggle menu button is clicked", async () => {
    const { container } = renderWithSimpleContent(MainLayout, baseOptions);
    const navbarToggleButton = container.querySelector(".dusk-navbar__toggle");

    await fireEvent.click(navbarToggleButton);

    expect(document.body.style.overflow).toBe("hidden");

    await fireEvent.click(navbarToggleButton);

    expect(document.body.style.overflow).toBe("auto");
  });
});
