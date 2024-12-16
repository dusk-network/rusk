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

  /**
   * Function to serialize an object containing BigInt values.
   * Converts BigInt values to strings with an 'n' suffix.
   *
   * @type {(value: any) => string}
   */
  let serializeProperty;

  /**
   * Function to deserialize a JSON string with BigInt values in string format.
   * Converts strings with an 'n' suffix back to BigInt values.
   *
   * @type {(value: string) => any}
   */
  let deserializeProperty;

  beforeEach(async () => {
    vi.resetModules();
    const settingsStoreModule = await import("../settingsStore");
    settingsStore = settingsStoreModule.default;
    settingsStoreContent = get(settingsStore);
    serializeProperty = settingsStoreModule.serializeProperty;
    deserializeProperty = settingsStoreModule.deserializeProperty;
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

      const expectedSettings = {
        ...settingsStoreContent,
        darkMode: false,
        gasLimit: `${settingsStoreContent.gasLimit}n`,
        gasPrice: `${settingsStoreContent.gasPrice}n`,
      };

      expect(localStorageSettings).toStrictEqual(expectedSettings);
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

    it("should expose a method to reset the gas settings to their defaults", () => {
      const newStateWithoutGas = {
        currency: "FAKE CURRENCY",
        darkMode: !settingsStoreContent.darkMode,
        dashboardTransactionLimit:
          settingsStoreContent.dashboardTransactionLimit + 1,
        hideStakingNotice: !settingsStoreContent.hideStakingNotice,
        language: "FAKE LANGUAGE",
        userId: "FAKE USER ID",
      };
      const newState = {
        ...newStateWithoutGas,
        gasLimit: settingsStoreContent.gasLimit * 2n,
        gasPrice: settingsStoreContent.gasPrice * 15n,
      };
      const expectedState = {
        ...settingsStoreContent,
        ...newStateWithoutGas,
      };

      settingsStore.set(newState);

      expect(get(settingsStore)).toBe(newState);

      settingsStore.resetGasSettings();

      expect(get(settingsStore)).toStrictEqual(expectedState);
      expect(
        JSON.parse(
          // @ts-ignore
          localStorage.getItem(`${CONFIG.LOCAL_STORAGE_APP_KEY}-preferences`)
        )
      ).toStrictEqual({
        ...expectedState,
        gasLimit: `${expectedState.gasLimit}n`,
        gasPrice: `${expectedState.gasPrice}n`,
      });
    });
  });

  describe("In a non browser environment", () => {
    it("should use its own defaults in place of the browser's settings", () => {
      expect(settingsStoreContent.language).toBe("en");
      expect(settingsStoreContent.darkMode).toBe(false);
    });
  });

  describe("BigInt serialization and deserialization", () => {
    it("should serialize BigInt values correctly", () => {
      const objWithBigInt = {
        gasLimit: BigInt(1000000000),
        gasPrice: BigInt(2000000000),
      };

      const serialized = serializeProperty(objWithBigInt);

      const expectedSerialized = JSON.stringify({
        gasLimit: "1000000000n",
        gasPrice: "2000000000n",
      });

      expect(serialized).toBe(expectedSerialized);
    });

    it("should deserialize BigInt values correctly", () => {
      const serialized = JSON.stringify({
        gasLimit: "1000000000n",
        gasPrice: "2000000000n",
      });

      const deserialized = deserializeProperty(serialized);

      const expectedDeserialized = {
        gasLimit: BigInt(1000000000),
        gasPrice: BigInt(2000000000),
      };

      expect(deserialized).toStrictEqual(expectedDeserialized);
    });
  });
});
