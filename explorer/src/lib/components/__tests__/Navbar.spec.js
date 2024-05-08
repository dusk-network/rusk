import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/svelte";
import * as appNavigation from "$app/navigation";

import { Navbar } from "..";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Navbar", () => {
  /** @type {(navigation: import("@sveltejs/kit").AfterNavigate) => void} */
  let afterNavigateCallback;

  const afterNavigateSpy = vi
    .spyOn(appNavigation, "afterNavigate")
    .mockImplementation((fn) => {
      afterNavigateCallback = fn;
    });

  afterEach(() => {
    cleanup();
    afterNavigateSpy.mockClear();
  });

  afterAll(() => {
    afterNavigateSpy.mockRestore();
  });

  it("renders the Navbar component", () => {
    const { container } = render(Navbar);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should hide the mobile menu after a navigation event", async () => {
    const { container } = render(Navbar);

    const btnMenuToggle = /** @type {HTMLButtonElement} */ (
      container.querySelector(".dusk-navbar__toggle")
    );
    const menu = /** @type {HTMLDivElement} */ (
      container.querySelector(".dusk-navbar__menu")
    );

    expect(menu).toHaveClass("dusk-navbar__menu--hidden");
    expect(btnMenuToggle).toHaveAttribute("aria-expanded", "false");

    await fireEvent.click(btnMenuToggle);

    expect(menu).not.toHaveClass("dusk-navbar__menu--hidden");
    expect(btnMenuToggle).toHaveAttribute("aria-expanded", "true");

    await act(() => {
      // @ts-expect-error we don't care for navigation details
      afterNavigateCallback();
    });

    expect(menu).toHaveClass("dusk-navbar__menu--hidden");
    expect(btnMenuToggle).toHaveAttribute("aria-expanded", "false");
  });
});
