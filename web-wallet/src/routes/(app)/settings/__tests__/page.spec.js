import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/svelte";

import mockedWalletStore from "../../__mocks__/mockedWalletStore";
import * as navigation from "$lib/navigation";
import { settingsStore, walletStore } from "$lib/stores";
import loginInfoStorage from "$lib/services/loginInfoStorage";

import Settings from "../+page.svelte";

/**
 * @param {HTMLElement} input
 * @param {*} value
 * @returns {Promise<boolean>}
 */
const fireInput = (input, value) =>
  fireEvent.input(input, { target: { value } });

/** @param {HTMLElement} element */
function asInput(element) {
  // eslint-disable-next-line no-extra-parens
  return /** @type {HTMLInputElement} */ (element);
}

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {import("$lib/stores/stores").WalletStore} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: {
      // @ts-ignore
      ...(await vi.importMock("$lib/stores/walletStore")).default,
      ...mockedWalletStore,
    },
  };
});

vi.useFakeTimers();

describe("Settings", () => {
  const initialWalletStoreState = structuredClone(
    mockedWalletStore.getMockedStoreValue()
  );
  const logoutSpy = vi.spyOn(navigation, "logout");

  beforeEach(() => {
    mockedWalletStore.setMockedStoreValue(initialWalletStoreState);
  });

  afterEach(() => {
    cleanup();
    logoutSpy.mockClear();
  });

  afterAll(() => {
    logoutSpy.mockRestore();
    vi.doUnmock("$lib/stores");
  });

  it("should render the settings page", () => {
    const { container } = render(Settings, {});

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the reset button while a sync is in progress", async () => {
    const { getByRole } = render(Settings);
    const resetButton = getByRole("button", { name: /reset/i });

    expect(resetButton).not.toHaveAttribute("disabled");
    expect(resetButton).toHaveAttribute("data-tooltip-disabled", "true");

    await act(() => {
      mockedWalletStore.setMockedStoreValue({
        ...initialWalletStoreState,
        isSyncing: true,
      });
    });

    expect(resetButton).toHaveAttribute("disabled");
    expect(resetButton).toHaveAttribute("data-tooltip-disabled", "false");
  });

  it('should disable the "Back" button if invalid gas limit or price are introduced', async () => {
    const { getByLabelText, getByRole } = render(Settings, {});
    const priceInput = asInput(getByLabelText(/price/i));
    const limitInput = asInput(getByLabelText(/limit/i));
    const backButton = getByRole("link");

    await fireInput(priceInput, 30000000);
    expect(backButton).toHaveAttribute("aria-disabled", "true");
    await fireInput(priceInput, 20000000);
    expect(backButton).toHaveAttribute("aria-disabled", "false");

    await fireInput(limitInput, 3000000000);
    expect(backButton).toHaveAttribute("aria-disabled", "true");
    await fireInput(limitInput, 20000000);
    expect(backButton).toHaveAttribute("aria-disabled", "false");
  });

  it("should reset wallet store and navigate to login page on clicking the Log Out button", async () => {
    const { getByRole } = render(Settings);

    const button = getByRole("button", { name: "Log out" });

    await fireEvent.click(button);
    await vi.advanceTimersToNextTimerAsync();

    expect(logoutSpy).toHaveBeenCalledTimes(1);
    expect(logoutSpy).toHaveBeenCalledWith(false);
  });

  describe("Resetting the wallet", () => {
    const clearDataSpy = vi
      .spyOn(walletStore, "clearLocalData")
      .mockResolvedValue(void 0);
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    const settingsResetSpy = vi.spyOn(settingsStore, "reset");
    const loginInfoStorageSpy = vi.spyOn(loginInfoStorage, "remove");

    afterEach(() => {
      clearDataSpy.mockClear();
      confirmSpy.mockClear();
      settingsResetSpy.mockClear();
      loginInfoStorageSpy.mockClear();
    });

    afterAll(() => {
      clearDataSpy.mockRestore();
      confirmSpy.mockRestore();
      settingsResetSpy.mockRestore();
      loginInfoStorageSpy.mockRestore();
    });

    it("should clear local data, settings, and login info before logging out the user if the reset button is clicked and the user confirms the operation", async () => {
      const { getByRole } = render(Settings);
      const resetButton = getByRole("button", { name: /reset/i });

      await fireEvent.click(resetButton);

      expect(confirmSpy).toHaveBeenCalledTimes(1);
      expect(clearDataSpy).toHaveBeenCalledTimes(1);

      await vi.advanceTimersToNextTimerAsync();

      expect(loginInfoStorageSpy).toHaveBeenCalledTimes(1);
      expect(settingsResetSpy).toHaveBeenCalledTimes(1);
      expect(logoutSpy).toHaveBeenCalledTimes(1);
      expect(logoutSpy).toHaveBeenCalledWith(false);
    });

    it("should do nothing if the user doesn't confirm the reset", async () => {
      confirmSpy.mockReturnValueOnce(false);

      const { getByRole } = render(Settings);
      const resetButton = getByRole("button", { name: /reset/i });

      await fireEvent.click(resetButton);

      expect(confirmSpy).toHaveBeenCalledTimes(1);
      expect(clearDataSpy).not.toHaveBeenCalled();

      await vi.advanceTimersToNextTimerAsync();

      expect(loginInfoStorageSpy).not.toHaveBeenCalled();
      expect(settingsResetSpy).not.toHaveBeenCalled();
      expect(logoutSpy).not.toHaveBeenCalled();
    });

    it("should show an error if clearing local data fails", async () => {
      clearDataSpy.mockRejectedValueOnce(new Error("Clear data error"));

      const { container, getByRole } = render(Settings);
      const resetButton = getByRole("button", { name: /reset/i });

      await fireEvent.click(resetButton);

      expect(confirmSpy).toHaveBeenCalledTimes(1);
      expect(clearDataSpy).toHaveBeenCalledTimes(1);

      await vi.advanceTimersToNextTimerAsync();

      expect(settingsResetSpy).not.toHaveBeenCalled();
      expect(logoutSpy).not.toHaveBeenCalled();
      expect(container.firstChild).toMatchSnapshot();
    });
  });
});
