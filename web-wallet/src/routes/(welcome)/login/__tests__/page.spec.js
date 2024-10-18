import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { generateMnemonic } from "bip39";
import { get } from "svelte/store";
import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

import { getKey, setKey } from "lamb";

import { getAsHTMLElement } from "$lib/dusk/test-helpers";
import * as navigation from "$lib/navigation";
import { settingsStore, walletStore } from "$lib/stores";
import {
  encryptMnemonic,
  getSeedFromMnemonic,
  profileGeneratorFrom,
} from "$lib/wallet";
import loginInfoStorage from "$lib/services/loginInfoStorage";

import Login from "../+page.svelte";

/** @param {HTMLElement} container */
function getTextInput(container) {
  // eslint-disable-next-line no-extra-parens
  return /** @type {HTMLInputElement} */ (
    container.querySelector("[type='password']")
  );
}

describe("Login", async () => {
  const mnemonic = generateMnemonic();
  const pwd = "some pwd";
  const loginInfo = await encryptMnemonic(mnemonic, pwd);
  const seed = getSeedFromMnemonic(mnemonic);
  const userId = await profileGeneratorFrom(seed)
    .default.then(getKey("address"))
    .then(String);

  const getErrorElement = () => document.querySelector(".login__error");
  const gotoSpy = vi.spyOn(navigation, "goto");

  /**
   * Sometimes a "DatabaseClosedError: Database has been closed" is
   * thrown when running this test (never happened running it in isolation).
   *
   * As I can't pinpoint what's causing it (all connections are opened before
   * db operations), I added the mocked implementation as here we don't care
   * about running `init` for real.
   */
  const initSpy = vi.spyOn(walletStore, "init").mockResolvedValue(void 0);

  afterEach(async () => {
    cleanup();
    gotoSpy.mockClear();
    initSpy.mockClear();
    settingsStore.reset();
    walletStore.reset();
  });

  afterAll(async () => {
    gotoSpy.mockRestore();
    initSpy.mockRestore();
  });

  describe("Mnemonic phrase workflow", () => {
    it("should render the login page and show the field to enter the mnemonic phrase, if there is no login info stored", () => {
      const { container } = render(Login, {});

      expect(container.firstChild).toMatchSnapshot();
    });

    it("should show an error message if the user enters an invalid mnemonic", async () => {
      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      expect(getErrorElement()).toBeNull();

      await fireEvent.input(textInput, { target: { value: "foo bar" } });
      await fireEvent.submit(form, { currentTarget: form });

      const errorElement = await vi.waitUntil(getErrorElement);
      const selectedText = textInput.value.substring(
        Number(textInput.selectionStart),
        Number(textInput.selectionEnd)
      );

      expect(initSpy).not.toHaveBeenCalled();
      expect(errorElement?.textContent).toMatch(/mnemonic/i);
      expect(textInput).toHaveFocus();
      expect(selectedText).toBe(textInput.value);
    });

    it("should redirect to the Restore flow if the user inputs a valid mnemonic with no prior wallet created", async () => {
      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, { target: { value: mnemonic } });
      await fireEvent.submit(form, { currentTarget: form });
      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(get(settingsStore).userId).toBe("");
      expect(initSpy).not.toHaveBeenCalled();
      expect(gotoSpy).toHaveBeenCalledWith("/setup/restore");
    });

    it("should redirect to the Restore flow the user inputs a valid mnemonic different from the last one used", async () => {
      const currentUserID = "some-user-id";
      settingsStore.update(setKey("userId", currentUserID));

      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, { target: { value: mnemonic } });
      await fireEvent.submit(form, { currentTarget: form });
      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(get(settingsStore).userId).toBe(currentUserID);
      expect(initSpy).not.toHaveBeenCalled();
      expect(gotoSpy).toHaveBeenCalledWith("/setup/restore");
    });

    it("should unlock the Wallet if the entered mnemonic is the last one used", async () => {
      settingsStore.update(setKey("userId", userId));

      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, { target: { value: mnemonic } });
      await fireEvent.submit(form, { currentTarget: form });
      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(get(settingsStore).userId).toBe(userId);
      expect(initSpy).toHaveBeenCalledTimes(1);
      expect(initSpy).toHaveBeenCalledWith(expect.any(ProfileGenerator));
      expect(gotoSpy).toHaveBeenCalledTimes(1);
      expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
    });

    it("should trim and lower case the entered mnemonic before validating it", async () => {
      settingsStore.update(setKey("userId", userId));

      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, {
        target: { value: `  \t${mnemonic.toUpperCase()} \t  ` },
      });
      await fireEvent.submit(form, { currentTarget: form });
      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(get(settingsStore).userId).toBe(userId);
      expect(initSpy).toHaveBeenCalledTimes(1);
      expect(initSpy).toHaveBeenCalledWith(expect.any(ProfileGenerator));
      expect(gotoSpy).toHaveBeenCalledTimes(1);
      expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
    });
  });

  describe("Password workflow", () => {
    beforeAll(() => {
      loginInfoStorage.set(loginInfo);

      return () => loginInfoStorage.remove();
    });

    it("should show the password field and the link to restore the wallet if there is login info stored", () => {
      const { container } = render(Login, {});

      expect(container.firstChild).toMatchSnapshot();
    });

    it("should show an error message if the user enters the wrong password", async () => {
      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      expect(getErrorElement()).toBeNull();

      await fireEvent.input(textInput, { target: { value: "foo bar" } });
      await fireEvent.submit(form, { currentTarget: form });

      const errorElement = await vi.waitUntil(getErrorElement);
      const selectedText = textInput.value.substring(
        Number(textInput.selectionStart),
        Number(textInput.selectionEnd)
      );

      expect(initSpy).not.toHaveBeenCalled();
      expect(errorElement?.textContent).toMatch(/password/i);
      expect(textInput).toHaveFocus();
      expect(selectedText).toBe(textInput.value);
    });

    /**
     * This is not a possible situation, in theory, but
     * the workflow is able to deal with it.
     */
    it("should redirect to the Restore flow if the user inputs the correct password with no prior wallet created", async () => {
      settingsStore.update(setKey("userId", ""));

      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, { target: { value: pwd } });
      await fireEvent.submit(form, { currentTarget: form });

      expect(getErrorElement()).toBeNull();

      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(getErrorElement()).toBeNull();
      expect(get(settingsStore).userId).toBe("");
      expect(initSpy).not.toHaveBeenCalled();
      expect(gotoSpy).toHaveBeenCalledTimes(1);
      expect(gotoSpy).toHaveBeenCalledWith("/setup/restore");
    });

    /**
     * This is not a possible situation, in theory, but
     * the workflow is able to deal with it.
     */
    it("should redirect to the Restore flow if the user inputs the correct password for a mnemonic different from the last one used", async () => {
      const currentUserID = "some-user-id";

      settingsStore.update(setKey("userId", currentUserID));

      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, { target: { value: pwd } });
      await fireEvent.submit(form, { currentTarget: form });

      expect(getErrorElement()).toBeNull();

      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(initSpy).not.toHaveBeenCalled();
      expect(gotoSpy).toHaveBeenCalledTimes(1);
      expect(gotoSpy).toHaveBeenCalledWith("/setup/restore");
    });

    it("should unlock the Wallet is the entered password is for the last used mnemonic", async () => {
      settingsStore.update(setKey("userId", userId));

      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, { target: { value: pwd } });
      await fireEvent.submit(form, { currentTarget: form });
      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(get(settingsStore).userId).toBe(userId);
      expect(initSpy).toHaveBeenCalledTimes(1);
      expect(initSpy).toHaveBeenCalledWith(expect.any(ProfileGenerator));
      expect(gotoSpy).toHaveBeenCalledTimes(1);
      expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
    });

    it("should trim the entered password before validating it", async () => {
      settingsStore.update(setKey("userId", userId));

      const { container } = render(Login, {});
      const form = getAsHTMLElement(container, "form");
      const textInput = getTextInput(container);

      await fireEvent.input(textInput, {
        target: { value: `  \t${pwd} \t  ` },
      });
      await fireEvent.submit(form, { currentTarget: form });
      await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

      expect(get(settingsStore).userId).toBe(userId);
      expect(initSpy).toHaveBeenCalledTimes(1);
      expect(initSpy).toHaveBeenCalledWith(expect.any(ProfileGenerator));
      expect(gotoSpy).toHaveBeenCalledTimes(1);
      expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
    });
  });
});
