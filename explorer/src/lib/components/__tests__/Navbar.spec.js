import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/svelte";
import * as appNavigation from "$app/navigation";

import { Navbar } from "..";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

/** @param {HTMLElement} container */
const getNotificationElement = (container) =>
  container.querySelector(".dusk-navbar__menu--search-notification");

/** @param {HTMLElement} container */
async function showSearchNotification(container) {
  const form = /** @type {HTMLFormElement} */ (container.querySelector("form"));
  const searchInput = /** @type {HTMLInputElement} */ (
    form.querySelector("input[type='text']")
  );

  searchInput.value = "foobar"; // invalid search

  await fireEvent.submit(form);
}

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

  it("should hide the search notification when its close button is clicked", async () => {
    const { container } = render(Navbar);

    expect(getNotificationElement(container)).toBeNull();

    await showSearchNotification(container);

    expect(getNotificationElement(container)).toBeInTheDocument();

    const btnClose = /** @type {HTMLButtonElement} */ (
      container.querySelector(".search-notification__header-action")
    );

    await fireEvent.click(btnClose);

    expect(getNotificationElement(container)).toBeNull();
  });

  it("should hide the search notification after a navigation event", async () => {
    const { container } = render(Navbar);

    expect(getNotificationElement(container)).toBeNull();

    await showSearchNotification(container);

    expect(getNotificationElement(container)).toBeInTheDocument();

    await act(() => {
      // @ts-expect-error we don't care for navigation details
      afterNavigateCallback();
    });

    expect(getNotificationElement(container)).toBeNull();
  });
});
