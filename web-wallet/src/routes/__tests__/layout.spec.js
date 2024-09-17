import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { networkStore, settingsStore } from "$lib/stores";

import MainLayout from "../+layout.svelte";

describe("Main layout", async () => {
  const isDarkMode = () => get(settingsStore).darkMode;
  const hasDarkClass = () =>
    document.documentElement.classList.contains("dark");
  const networkDisconnectSpy = vi
    .spyOn(networkStore, "disconnect")
    .mockResolvedValue(undefined);

  afterEach(() => {
    cleanup();
    settingsStore.reset();
    networkDisconnectSpy.mockClear();
  });

  afterAll(() => {
    networkDisconnectSpy.mockRestore();
  });

  it("should render the main layout and disconnect from the network when dismounting", () => {
    const { container, unmount } = render(MainLayout);

    expect(container).toMatchSnapshot();

    unmount();

    expect(networkDisconnectSpy).toHaveBeenCalledTimes(1);
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
