import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { get } from "svelte/store";

describe("Settings store", () => {
  const languageSpy = vi
    .spyOn(navigator, "language", "get")
    .mockReturnValue("it-IT");

  const originalMatchMedia = window.matchMedia;

  Object.defineProperty(window, "matchMedia", {
    /** @param {MediaQueryList} query */
    value: (query) => ({
      addEventListener: () => {},
      dispatchEvent: () => {},
      matches: true,
      media: query,
      onchange: null,
      removeEventListener: () => {},
    }),
    writable: true,
  });

  /** @type {SettingsStore} */
  let settingsStore;

  /** @type {SettingsStoreContent} */
  let settingsStoreContent;

  beforeEach(async () => {
    vi.resetModules();
    settingsStore = (await import("../settingsStore")).default;
    settingsStoreContent = get(settingsStore);
  });

  afterAll(() => {
    languageSpy.mockRestore();

    window.matchMedia = originalMatchMedia;
  });

  describe("In a browser environment", () => {
    vi.doMock("$app/environment", async (importOriginal) => {
      /** @type {typeof import("$app/environment")} */
      const original = await importOriginal();

      return {
        ...original,
        browser: true,
      };
    });

    afterEach(() => {
      localStorage.clear();
    });

    afterAll(() => {
      vi.doUnmock("$app/environment");
    });

    it("should get the browser's settings for dark mode and language and use them along the other defaults", async () => {
      expect(settingsStoreContent.language).toBe("it-IT");
      expect(settingsStoreContent.darkMode).toBe(true);
    });

    it("should persist its values in local storage", async () => {
      settingsStore.update((store) => ({
        ...store,
        darkMode: false,
      }));

      const localStorageSettings = JSON.parse(
        // @ts-expect-error
        localStorage.getItem(`${CONFIG.LOCAL_STORAGE_APP_KEY}-preferences`)
      );

      expect(localStorageSettings).toStrictEqual({
        ...settingsStoreContent,
        darkMode: false,
      });
    });

    it("should expose a method to reset the store to its initial state", () => {
      const newState = {
        ...settingsStoreContent,
        darkMode: false,
        foo: "bar",
      };

      settingsStore.set(newState);

      expect(get(settingsStore)).toBe(newState);

      settingsStore.reset();

      expect(get(settingsStore)).toBe(settingsStoreContent);
    });
  });

  describe("In a non browser environment", () => {
    it("should use its own defaults in place of the browser's settings", () => {
      expect(settingsStoreContent.language).toBe("en");
      expect(settingsStoreContent.darkMode).toBe(false);
    });
  });
});
