import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { tick } from "svelte";
import { get } from "svelte/store";
import * as SvelteKit from "@sveltejs/kit";
import { getKey, setKey } from "lamb";
import * as bip39 from "bip39";
import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

import { networkStore, settingsStore, walletStore } from "$lib/stores";
import * as walletLib from "$lib/wallet";
import * as navigation from "$lib/navigation";
import loginInfoStorage from "$lib/services/loginInfoStorage";
import * as shuffleArray from "$lib/dusk/array";

import Create from "../+page.svelte";
import { load } from "../+page.js";

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

describe("Create", async () => {
  const currentBlockHeight = 1536n;
  const mnemonic =
    "cart dad sail wreck robot grit combine noble rap farm slide sad";
  const mnemonicShuffled = [
    "grit",
    "wreck",
    "cart",
    "dad",
    "rap",
    "sail",
    "robot",
    "combine",
    "noble",
    "slide",
    "sad",
    "farm",
  ];
  const pwd = "passwordpassword";
  const seed = walletLib.getSeedFromMnemonic(mnemonic);
  const userId = await walletLib
    .profileGeneratorFrom(seed)
    .then(getKey("default"))
    .then(getKey("address"))
    .then(String);

  const baseProps = {
    data: { currentBlockHeight },
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  const generateMnemonicSpy = vi
    .spyOn(bip39, "generateMnemonic")
    .mockReturnValue(mnemonic);
  const shuffleArraySpy = vi
    .spyOn(shuffleArray, "shuffleArray")
    .mockReturnValue(mnemonicShuffled);
  const gotoSpy = vi.spyOn(navigation, "goto");
  const settingsResetSpy = vi.spyOn(settingsStore, "reset");
  const clearAndInitSpy = vi
    .spyOn(walletStore, "clearLocalDataAndInit")
    .mockResolvedValue(undefined);
  const initWalletSpy = vi.spyOn(walletLib, "initializeWallet");

  afterEach(() => {
    cleanup();
    settingsStore.reset();

    generateMnemonicSpy.mockClear();
    shuffleArraySpy.mockClear();
    clearAndInitSpy.mockClear();
    gotoSpy.mockClear();
    settingsResetSpy.mockClear();
    initWalletSpy.mockClear();
  });

  afterAll(() => {
    generateMnemonicSpy.mockRestore();
    shuffleArraySpy.mockRestore();
    clearAndInitSpy.mockRestore();
    gotoSpy.mockRestore();
    settingsResetSpy.mockRestore();
    initWalletSpy.mockRestore();
  });

  describe("Create page load", () => {
    const blockHeightSpy = vi
      .spyOn(networkStore, "getCurrentBlockHeight")
      .mockResolvedValue(currentBlockHeight);

    afterEach(() => {
      blockHeightSpy.mockClear();
    });

    afterAll(() => {
      blockHeightSpy.mockRestore();
    });

    it('should retrieve the current block height and pass it to the page as "page data"', async () => {
      // @ts-ignore we don't care for load parameters here
      await expect(load()).resolves.toStrictEqual({ currentBlockHeight });
    });

    it("should call the svelte kit error if the current block height can't be retrieved", async () => {
      const svelteKitErrorSpy = vi.spyOn(SvelteKit, "error");

      blockHeightSpy.mockRejectedValueOnce(new Error("some error"));

      // @ts-ignore we don't care for load parameters here
      await expect(load()).rejects.toThrow();

      expect(svelteKitErrorSpy).toHaveBeenCalledTimes(1);
      expect(svelteKitErrorSpy).toHaveBeenCalledWith(500, expect.any(String));

      svelteKitErrorSpy.mockRestore();
    });
  });

  it("should render the Existing Wallet notice step of the Create flow if there is a userId saved in localStorage", () => {
    settingsStore.update(setKey("userId", userId));

    const { container } = render(Create, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the Terms of Service step of the Create flow if there is no userId saved in localStorage", () => {
    const { container } = render(Create, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the `Securely store your mnemonic phrase!` agreement step after the ToS", async () => {
    const { container, getByRole } = render(Create, baseOptions);

    const mathRandomSpy = vi.spyOn(Math, "random").mockReturnValue(42);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    expect(container.firstChild).toMatchSnapshot();

    mathRandomSpy.mockRestore();
  });

  it("should not allow the user proceed unless both agreement checks are selected on the `Securely store your mnemonic phrase!` step", async () => {
    const { getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    const firstCheckbox = getAllByRole("checkbox")[0];
    const secondCheckbox = getAllByRole("checkbox")[1];
    const nextButton = getByRole("button", { name: "Next" });

    // Select the first checkbox
    await fireEvent.click(firstCheckbox);

    // Ensure Next is disabled
    expect(nextButton).toBeDisabled();

    // Unselect the first checkbox
    await fireEvent.click(firstCheckbox);

    // Select the second checkbox
    await fireEvent.click(secondCheckbox);

    // Ensure Next is disabled
    expect(getByRole("button", { name: "Next" })).toBeDisabled();

    // Select first checkbox too
    await fireEvent.click(firstCheckbox);

    // Ensure Next is enabled
    expect(nextButton).toBeEnabled();
  });

  it("correctly renders the Mnemonic Preview page", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(container.firstChild).toMatchSnapshot();
  });

  it("correctly renders the Mnemonic Verification page", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(container.firstChild).toMatchSnapshot();
  });

  it("doesn't let the user proceed if they have entered mismatching Mnemonic", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    const wordButtonsWrapper = container.getElementsByClassName(
      "dusk-mnemonic__validate-actions-wrapper"
    )[0];

    const wordButtons = Array.from(wordButtonsWrapper.children);

    wordButtons.forEach(async (button) => {
      await fireEvent.click(button);
    });

    await tick();

    expect(container.firstChild).toMatchSnapshot();

    expect(getByRole("button", { name: "Next" })).toBeDisabled();
  });

  it("lets the user proceed if they have entered a matching Mnemonic", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    const mnemonicSplit = mnemonic.split(" ");

    mnemonicSplit.forEach(async (word) => {
      await fireEvent.click(getByRole("button", { name: word }));
    });

    await tick();

    expect(container.firstChild).toMatchSnapshot();

    expect(getByRole("button", { name: "Next" })).toBeEnabled();
  });

  it("ensures that the Undo button on the Mnemonic Validate step works as expected", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    const mnemonicSplit = mnemonic.split(" ");

    mnemonicSplit.forEach(async (word) => {
      await fireEvent.click(getByRole("button", { name: word }));
    });

    await tick();

    await fireEvent.click(getByRole("button", { name: "Undo" }));

    expect(container.firstChild).toMatchSnapshot();

    expect(getByRole("button", { name: "Next" })).toBeDisabled();
  });

  it("ensures the Password step renders as expected", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    const mnemonicSplit = mnemonic.split(" ");

    mnemonicSplit.forEach(async (word) => {
      await fireEvent.click(getByRole("button", { name: word }));
    });

    await tick();

    await fireEvent.click(getByRole("button", { name: "Next" }));

    // Password enabled
    expect(container.firstChild).toMatchSnapshot();

    await fireEvent.click(getByRole("switch"));

    // Password disabled
    expect(container.firstChild).toMatchSnapshot();
  });

  it("ensures the Network Syncing step renders as expected", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    const mnemonicSplit = mnemonic.split(" ");

    mnemonicSplit.forEach(async (word) => {
      await fireEvent.click(getByRole("button", { name: word }));
    });

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(container.firstChild).toMatchSnapshot();
  });

  it("ensures the All Done step renders as expected", async () => {
    const { container, getByRole, getAllByRole } = render(Create, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Accept" }));

    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    const mnemonicSplit = mnemonic.split(" ");

    mnemonicSplit.forEach(async (word) => {
      await fireEvent.click(getByRole("button", { name: word }));
    });

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));
    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(container.firstChild).toMatchSnapshot();

    /*
     * We wait for the `clearLocalDataAndInit` call inside `initializeWallet`
     * as the promise is not awaited and can happen after the test ends
     * invalidating the checks of other tests.
     */
    await vi.waitUntil(() => clearAndInitSpy.mock.calls.length === 1);
  });

  it("should initialize the wallet without setting a password", async () => {
    const { getByRole, getAllByRole } = render(Create, baseOptions);

    // ToS step
    await fireEvent.click(getByRole("button", { name: "Accept" }));

    // Mnemonic Agreement step
    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));

    // Mnemonic Generate step
    await fireEvent.click(getByRole("button", { name: "Next" }));

    // Mnemonic Validate step
    const mnemonicSplit = mnemonic.split(" ");

    mnemonicSplit.forEach(async (word) => {
      await fireEvent.click(getByRole("button", { name: word }));
    });

    await fireEvent.click(getByRole("button", { name: "Next" }));

    // Set Password step
    await fireEvent.click(getByRole("switch"));

    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(loginInfoStorage.get()).toBeNull();

    // Network Syncing step
    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(settingsResetSpy).toHaveBeenCalledTimes(1);
    expect(initWalletSpy).toHaveBeenCalledTimes(1);
    expect(initWalletSpy).toHaveBeenCalledWith(mnemonic, currentBlockHeight);

    await vi.waitUntil(() => clearAndInitSpy.mock.calls.length === 1);

    expect(get(settingsStore).walletCreationBlockHeight).toBe(
      currentBlockHeight
    );
    expect(clearAndInitSpy).toHaveBeenCalledTimes(1);
    expect(clearAndInitSpy).toHaveBeenCalledWith(
      expect.any(ProfileGenerator),
      currentBlockHeight
    );

    // All Done step
    await fireEvent.click(getByRole("button", { name: "Next" }));

    await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

    expect(gotoSpy).toHaveBeenCalledTimes(1);
    expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
  });

  it("should initialize the wallet encrypted mnemonic saved in localStorage", async () => {
    const { getByPlaceholderText, getByRole, getAllByRole } = render(
      Create,
      baseOptions
    );

    // ToS step
    await fireEvent.click(getByRole("button", { name: "Accept" }));

    // Mnemonic Agreement step
    await fireEvent.click(getAllByRole("checkbox")[0]);
    await fireEvent.click(getAllByRole("checkbox")[1]);

    await fireEvent.click(getByRole("button", { name: "Next" }));

    // Mnemonic Generate step
    await fireEvent.click(getByRole("button", { name: "Next" }));

    // Mnemonic Validate step
    const mnemonicSplit = mnemonic.split(" ");

    mnemonicSplit.forEach(async (word) => {
      await fireEvent.click(getByRole("button", { name: word }));
    });

    await fireEvent.click(getByRole("button", { name: "Next" }));

    // Set Password step
    expect(loginInfoStorage.get()).toBeNull();

    await fireInput(asInput(getByPlaceholderText("Set Password")), pwd);
    await fireInput(asInput(getByPlaceholderText("Confirm Password")), pwd);

    expect(loginInfoStorage.get()).toBeNull();

    await fireEvent.click(getByRole("button", { name: "Next" }));
    await vi.waitFor(() => {
      expect(loginInfoStorage.get()).not.toBeNull();
    });

    // Network Syncing step
    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(settingsResetSpy).toHaveBeenCalledTimes(1);
    expect(initWalletSpy).toHaveBeenCalledTimes(1);
    expect(initWalletSpy).toHaveBeenCalledWith(mnemonic, currentBlockHeight);

    await vi.waitUntil(() => clearAndInitSpy.mock.calls.length === 1);

    expect(get(settingsStore).walletCreationBlockHeight).toBe(
      currentBlockHeight
    );
    expect(clearAndInitSpy).toHaveBeenCalledTimes(1);
    expect(clearAndInitSpy).toHaveBeenCalledWith(
      expect.any(ProfileGenerator),
      currentBlockHeight
    );

    // All Done step
    await fireEvent.click(getByRole("button", { name: "Next" }));

    await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

    expect(gotoSpy).toHaveBeenCalledTimes(1);
    expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
  });
});
