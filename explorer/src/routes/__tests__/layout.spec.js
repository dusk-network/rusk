import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent } from "@testing-library/svelte";
import { get } from "svelte/store";
import { appStore } from "$lib/stores";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

import MainLayout from "../+layout.svelte";

describe("Main layout", () => {
  const baseOptions = { props: {}, target: document.body };
 
  afterEach(() => {
    cleanup();
  });

  it("should render the app's main layout", () => {
    const { container } = renderWithSimpleContent(MainLayout, baseOptions);
    expect(container).toMatchSnapshot();
  });

  it("should change the overflow of document's body when the navbar toggle menu button is clicked", async () => {
    const { container } = renderWithSimpleContent(MainLayout, baseOptions);
    const navbarToggleButton = /** @type {HTMLButtonElement} */ (
      container.querySelector(".dusk-navbar__toggle")
    );

    await fireEvent.click(navbarToggleButton);

    expect(document.body.style.overflow).toBe("hidden");

    await fireEvent.click(navbarToggleButton);

    expect(document.body.style.overflow).toBe("auto");
  });

  it('should add and remove the "dark" class name to the `html` element when the `darkMode` value changes in the settings store', async () => {
    const isDarkMode = () => get(appStore).darkMode;
    const hasDarkClass = () => document.documentElement.classList.contains("dark");

    expect(isDarkMode()).toBe(false);
    expect(hasDarkClass()).toBe(false);

    appStore.setTheme(true);

    expect(isDarkMode()).toBe(true);
    expect(hasDarkClass()).toBe(true);

    appStore.setTheme(false);

    expect(isDarkMode()).toBe(false);
    expect(hasDarkClass()).toBe(false);
  });
});