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
import { get } from "svelte/store";
import { setKey } from "lamb";

import mockedWalletStore from "$lib/__mocks__/mockedWalletStore";
import * as navigation from "$lib/navigation";
import {
  gasStore,
  networkStore,
  settingsStore,
  walletStore,
} from "$lib/stores";
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
  /** @type {WalletStore} */
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

  beforeEach(async () => {
    await networkStore.connect();
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

  it("should render the settings page displaying the status of the network", async () => {
    const { container } = render(Settings, {});

    expect(container.firstChild).toMatchSnapshot();

    await networkStore.disconnect();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should show the wallet creation block height if it's greater than zero", () => {
    settingsStore.update(setKey("walletCreationBlockHeight", 123n));

    const { container, getByDisplayValue } = render(Settings, {});
    const creationBlockInput = getByDisplayValue("123");

    expect(creationBlockInput).toBeInTheDocument();
    expect(container.firstChild).toMatchSnapshot();

    settingsStore.reset();
  });

  it("should disable the reset wallet button while a sync is in progress", async () => {
    const { getByRole } = render(Settings);
    const resetButton = getByRole("button", { name: /reset wallet/i });

    expect(resetButton).not.toHaveAttribute("disabled");
    expect(resetButton).toHaveAttribute("data-tooltip-disabled", "true");

    await act(() => {
      mockedWalletStore.setMockedStoreValue({
        ...initialWalletStoreState,
        syncStatus: {
          ...initialWalletStoreState.syncStatus,
          isInProgress: true,
        },
      });
    });

    expect(resetButton).toHaveAttribute("disabled");
    expect(resetButton).toHaveAttribute("data-tooltip-disabled", "false");
  });

  it('should disable the "Back" button if invalid gas limit or price are introduced', async () => {
    const { gasLimitLower, gasLimitUpper, gasPriceLower } = get(gasStore);
    const { getByLabelText, getByRole } = render(Settings, {});
    const priceInput = asInput(getByLabelText(/price/i));
    const limitInput = asInput(getByLabelText(/limit/i));
    const backButton = getByRole("link", { name: /back/i });

    await fireInput(priceInput, String(gasPriceLower - 1n));
    expect(backButton).toHaveAttribute("aria-disabled", "true");
    await fireInput(priceInput, gasPriceLower.toString());
    expect(backButton).toHaveAttribute("aria-disabled", "false");

    await fireInput(limitInput, String(gasLimitLower - 1n));
    expect(backButton).toHaveAttribute("aria-disabled", "true");
    await fireInput(limitInput, gasLimitLower.toString());
    expect(backButton).toHaveAttribute("aria-disabled", "false");
    await fireInput(limitInput, String(gasLimitUpper + 1n));
    expect(backButton).toHaveAttribute("aria-disabled", "true");
    await fireInput(limitInput, gasLimitUpper.toString());
    expect(backButton).toHaveAttribute("aria-disabled", "false");
  });

  it("should reset wallet store and navigate to landing page on clicking the Lock Wallet button", async () => {
    const { getByRole } = render(Settings);

    const button = getByRole("button", { name: "Lock Wallet" });

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

    beforeEach(() => {
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

    it("should clear local data, settings, and login info before logging out the user if the reset wallet button is clicked and the user confirms the operation", async () => {
      const { getByRole } = render(Settings);
      const resetButton = getByRole("button", { name: /reset wallet/i });

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
      const resetButton = getByRole("button", { name: /reset wallet/i });

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
      const resetButton = getByRole("button", { name: /reset wallet/i });

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
