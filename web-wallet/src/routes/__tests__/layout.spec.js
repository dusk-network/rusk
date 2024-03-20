import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { settingsStore } from "$lib/stores";

import MainLayout from "../+layout.svelte";

describe("Main layout", () => {
  const isDarkMode = () => get(settingsStore).darkMode;
  const hasDarkClass = () =>
    document.documentElement.classList.contains("dark");

  afterEach(() => {
    cleanup();
    settingsStore.reset();
  });

  it("should render the main layout", () => {
    const { container } = render(MainLayout);

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should add and remove the "dark" class name to the `html` element when the `darkMode` value changes in thesettings store', () => {
    expect(isDarkMode()).toBe(false);
    expect(hasDarkClass()).toBe(false);

    settingsStore.update((store) => ({ ...store, darkMode: true }));

    expect(isDarkMode()).toBe(true);
    expect(hasDarkClass()).toBe(true);

    settingsStore.reset();

    expect(isDarkMode()).toBe(false);
    expect(hasDarkClass()).toBe(false);
  });
});
