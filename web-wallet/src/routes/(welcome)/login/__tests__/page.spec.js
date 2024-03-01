import {
	afterAll,
	afterEach,
	beforeAll,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { generateMnemonic } from "bip39";
import { get } from "svelte/store";
import { Wallet } from "@dusk-network/dusk-wallet-js";
import { setKey } from "lamb";

import { addresses } from "$lib/mock-data";
import { getAsHTMLElement } from "$lib/dusk/test-helpers";
import * as navigation from "$lib/navigation";
import { settingsStore, walletStore } from "$lib/stores";
import { encryptMnemonic, getSeedFromMnemonic } from "$lib/wallet";
import loginInfoStorage from "$lib/services/loginInfoStorage";
import * as walletService from "$lib/services/wallet";

import Login from "../+page.svelte";

/** @param {HTMLElement} container */
function getTextInput (container) {
	// eslint-disable-next-line no-extra-parens
	return /** @type {HTMLInputElement} */ (container.querySelector("[type='password']"));
}

describe("Login", async () => {
	const walletGetPsksSpy = vi.spyOn(Wallet.prototype, "getPsks").mockResolvedValue(addresses);
	const walletResetSpy = vi.spyOn(Wallet.prototype, "reset").mockResolvedValue(void 0);
	const mnemonic = generateMnemonic();
	const pwd = "some pwd";
	const loginInfo = await encryptMnemonic(mnemonic, pwd);
	const seed = getSeedFromMnemonic(mnemonic);
	const userId = (await new Wallet(seed).getPsks())[0];
	const getErrorElement = () => document.querySelector(".login__error");
	const getWalletSpy = vi.spyOn(walletService, "getWallet");
	const gotoSpy = vi.spyOn(navigation, "goto");
	const initSpy = vi.spyOn(walletStore, "init");
	const settingsResetSpy = vi.spyOn(settingsStore, "reset");
	const confirmSpy = vi.spyOn(window, "confirm");

	afterEach(async () => {
		cleanup();
		confirmSpy.mockClear();
		getWalletSpy.mockClear();
		gotoSpy.mockClear();
		initSpy.mockClear();
		settingsStore.reset();
		settingsResetSpy.mockClear();
		walletGetPsksSpy.mockClear();
		walletResetSpy.mockClear();
		walletStore.reset();
	});

	afterAll(() => {
		confirmSpy.mockRestore();
		getWalletSpy.mockRestore();
		gotoSpy.mockRestore();
		initSpy.mockRestore();
		settingsResetSpy.mockRestore();
		walletGetPsksSpy.mockRestore();
		walletResetSpy.mockRestore();
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

			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(initSpy).not.toHaveBeenCalled();
			expect(errorElement?.textContent).toMatch(/mnemonic/i);
			expect(textInput).toHaveFocus();
			expect(selectedText).toBe(textInput.value);
		});

		it("should clear local data and redirect to the dashboard if the user inputs a valid mnemonic with no prior wallet created", async () => {
			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: mnemonic } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

			expect(confirmSpy).not.toHaveBeenCalled();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).toHaveBeenCalledTimes(1);
			expect(settingsResetSpy).toHaveBeenCalledTimes(1);
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
		});

		it("should trigger a notice if the user inputs a valid mnemonic different from the last one used, then clear local data and redirect to dashboard if the notice is accepted", async () => {
			settingsStore.update(setKey("userId", "some-user-id"));
			confirmSpy.mockReturnValue(true);

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: mnemonic } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => confirmSpy.mock.calls.length > 0);

			expect(confirmSpy).toHaveBeenCalledOnce();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).toHaveBeenCalledTimes(1);
			expect(settingsResetSpy).toHaveBeenCalledTimes(1);
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
		});

		it("should trigger a notice if the user inputs a valid mnemonic different from the last one used and show an error if the notice is declined", async () => {
			settingsStore.update(setKey("userId", "some-user-id"));
			confirmSpy.mockReturnValue(false);

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			expect(getErrorElement()).toBeNull();

			await fireEvent.input(textInput, { target: { value: mnemonic } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => confirmSpy.mock.calls.length > 0);

			expect(confirmSpy).toHaveBeenCalledOnce();

			const errorElement = await vi.waitUntil(getErrorElement);
			const selectedText = textInput.value.substring(
				Number(textInput.selectionStart),
				Number(textInput.selectionEnd)
			);

			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).not.toBe(userId);
			expect(initSpy).not.toHaveBeenCalled();
			expect(gotoSpy).not.toHaveBeenCalled();
			expect(errorElement?.textContent).toBe("Existing wallet detected");
			expect(textInput).toHaveFocus();
			expect(selectedText).toBe(textInput.value);
		});

		it("should not show a notice and should not clear local data if the entered mnemonic is the last one used", async () => {
			settingsStore.update(setKey("userId", userId));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: mnemonic } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

			expect(confirmSpy).not.toHaveBeenCalled();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
		});

		it("should show an error if the clearing of local data fails", async () => {
			settingsStore.update(setKey("userId", ""));

			const errorMessage = "Failed to delete data";

			walletResetSpy.mockRejectedValueOnce(new Error(errorMessage));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			expect(getErrorElement()).toBeNull();

			await fireEvent.input(textInput, { target: { value: mnemonic } });
			await fireEvent.submit(form, { currentTarget: form });

			const errorElement = await vi.waitUntil(getErrorElement);
			const selectedText = textInput.value.substring(
				Number(textInput.selectionStart),
				Number(textInput.selectionEnd)
			);

			expect(confirmSpy).not.toHaveBeenCalled();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).toHaveBeenCalledTimes(1);
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).not.toBe(userId);
			expect(initSpy).not.toHaveBeenCalled();
			expect(gotoSpy).not.toHaveBeenCalled();
			expect(errorElement?.textContent).toBe(errorMessage);
			expect(textInput).toHaveFocus();
			expect(selectedText).toBe(textInput.value);
		});

		it("should trim and lower case the entered mnemonic before validating it", async () => {
			settingsStore.update(setKey("userId", userId));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: `  \t${mnemonic.toUpperCase()} \t  ` } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

			expect(confirmSpy).not.toHaveBeenCalled();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
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

			expect(confirmSpy).not.toHaveBeenCalled();
			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(initSpy).not.toHaveBeenCalled();
			expect(errorElement?.textContent).toMatch(/password/i);
			expect(textInput).toHaveFocus();
			expect(selectedText).toBe(textInput.value);
		});

		/**
		 * This is not a possible situation, in theory, but
		 * the workflow is able to deal with it.
		 */
		it("should clear local data and redirect to the dashboard if the user inputs the correct password with no prior wallet created", async () => {
			settingsStore.update(setKey("userId", ""));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });

			expect(getErrorElement()).toBeNull();

			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

			expect(getErrorElement()).toBeNull();
			expect(confirmSpy).not.toHaveBeenCalled();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).toHaveBeenCalledTimes(1);
			expect(settingsResetSpy).toHaveBeenCalledTimes(1);
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
		});

		/**
		 * This is not a possible situation, in theory, but
		 * the workflow is able to deal with it.
		 */
		it("should ask to clear local data and redirect to the dashboard if the user inputs the correct password for a mnemonic different from the last one used", async () => {
			settingsStore.update(setKey("userId", "some-fake-id"));
			confirmSpy.mockReturnValue(true);

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });

			expect(getErrorElement()).toBeNull();

			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

			expect(confirmSpy).toHaveBeenCalledOnce();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).toHaveBeenCalledTimes(1);
			expect(settingsResetSpy).toHaveBeenCalledTimes(1);
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
		});

		/**
		 * This is not a possible situation, in theory, but
		 * the workflow is able to deal with it.
		 */
		it("should ask to clear local data and show an error if the user inputs the correct password for a mnemonic different from the last one used and decline the notice", async () => {
			settingsStore.update(setKey("userId", "some-fake-id"));
			confirmSpy.mockReturnValue(false);

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			expect(getErrorElement()).toBeNull();

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => confirmSpy.mock.calls.length > 0);

			expect(confirmSpy).toHaveBeenCalledOnce();

			const errorElement = await vi.waitUntil(getErrorElement);
			const selectedText = textInput.value.substring(
				Number(textInput.selectionStart),
				Number(textInput.selectionEnd)
			);

			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).not.toBe(userId);
			expect(initSpy).not.toHaveBeenCalled();
			expect(gotoSpy).not.toHaveBeenCalled();
			expect(errorElement?.textContent).toBe("Existing wallet detected");
			expect(textInput).toHaveFocus();
			expect(selectedText).toBe(textInput.value);
		});

		it("should not show a notice and clear local data if the entered password is for the last used mnemonic", async () => {
			settingsStore.update(setKey("userId", userId));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

			expect(confirmSpy).not.toHaveBeenCalled();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
		});

		it("should show an error if the clearing of local data fails", async () => {
			settingsStore.update(setKey("userId", "some-fake-id"));
			confirmSpy.mockReturnValue(true);

			const errorMessage = "Failed to delete data";

			walletResetSpy.mockRejectedValueOnce(new Error(errorMessage));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			expect(getErrorElement()).toBeNull();

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => confirmSpy.mock.calls.length > 0);

			const errorElement = await vi.waitUntil(getErrorElement);
			const selectedText = textInput.value.substring(
				Number(textInput.selectionStart),
				Number(textInput.selectionEnd)
			);

			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).toHaveBeenCalledTimes(1);
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).not.toBe(userId);
			expect(initSpy).not.toHaveBeenCalled();
			expect(gotoSpy).not.toHaveBeenCalled();
			expect(errorElement?.textContent).toBe(errorMessage);
			expect(textInput).toHaveFocus();
			expect(selectedText).toBe(textInput.value);
		});

		it("should trim the entered password before validating it", async () => {
			settingsStore.update(setKey("userId", userId));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: `  \t${pwd} \t  ` } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

			expect(confirmSpy).not.toHaveBeenCalled();
			expect(getWalletSpy).toHaveBeenCalledTimes(1);
			expect(getWalletSpy).toHaveBeenCalledWith(seed);
			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(get(settingsStore).userId).toBe(userId);
			expect(initSpy).toHaveBeenCalledTimes(1);
			expect(initSpy).toHaveBeenCalledWith(expect.any(Wallet));
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
		});
	});
});
