import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

import { settingsStore, walletStore } from "$lib/stores";

import { initializeWallet } from "..";

describe("initializeWallet", () => {
  const settingsResetSpy = vi
    .spyOn(settingsStore, "reset")
    .mockReturnValue(undefined);
  const clearWalletAndInitSpy = vi
    .spyOn(walletStore, "clearLocalDataAndInit")
    .mockResolvedValue(undefined);

  afterEach(() => {
    settingsResetSpy.mockClear();
    clearWalletAndInitSpy.mockClear();
  });

  afterAll(() => {
    settingsResetSpy.mockRestore();
    clearWalletAndInitSpy.mockRestore();
  });

  it("should clear the settings store and initialize a new wallet", async () => {
    const mnemonic =
      "cart dad sail wreck robot grit combine noble rap farm slide sad";
    const from = 45n;

    await initializeWallet(mnemonic, from);

    expect(settingsResetSpy).toHaveBeenCalledTimes(1);
    expect(clearWalletAndInitSpy).toHaveBeenCalledTimes(1);
    expect(clearWalletAndInitSpy).toHaveBeenCalledWith(
      expect.any(ProfileGenerator),
      from
    );
  });
});
