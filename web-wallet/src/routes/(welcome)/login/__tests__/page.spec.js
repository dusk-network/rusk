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
import * as appNavigation from "$app/navigation";
import { get } from "svelte/store";
import { Wallet } from "@dusk-network/dusk-wallet-js";
import { setKey } from "lamb";

import { addresses } from "$lib/mock-data";
import { getAsHTMLElement } from "$lib/dusk/test-helpers";
import { settingsStore, walletStore } from "$lib/stores";
import { encryptMnemonic, getSeedFromMnemonic } from "$lib/wallet";
import loginInfoStorage from "$lib/services/loginInfoStorage";
import * as walletService from "$lib/services/wallet";

import Login from "../+page.svelte";

/** @param {HTMLElement} container */
function getTextInput (container) {
	// eslint-disable-next-line no-extra-parens
	return /** @type {HTMLInputElement} */ (container.querySelector("[type='password'"));
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
	const gotoSpy = vi.spyOn(appNavigation, "goto");
	const initSpy = vi.spyOn(walletStore, "init");
	const settingsResetSpy = vi.spyOn(settingsStore, "reset");

	afterEach(() => {
		cleanup();
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

		it("should clear local data and redirect to the dashboard if the user inputs a valid mnemonic different from the last one used", async () => {
			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: mnemonic } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

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

		it("should not clear local data if the entered mnemonic is the last one used", async () => {
			settingsStore.update(setKey("userId", userId));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: mnemonic } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

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

		it("should trim the entered mnemonic before validating it", async () => {
			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: `  \t${mnemonic} \t  ` } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

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

			expect(walletResetSpy).not.toHaveBeenCalled();
			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(initSpy).not.toHaveBeenCalled();
			expect(errorElement?.textContent).toMatch(/password/i);
			expect(textInput).toHaveFocus();
			expect(selectedText).toBe(textInput.value);
		});

		it("should clear local data and redirect to the dashboard if the user inputs the correct password different from the last one used", async () => {
			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

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

		it("should not clear local data if the entered mnemonic is the last one used", async () => {
			settingsStore.update(setKey("userId", userId));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

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
			const errorMessage = "Failed to delete data";

			walletResetSpy.mockRejectedValueOnce(new Error(errorMessage));

			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			expect(getErrorElement()).toBeNull();

			await fireEvent.input(textInput, { target: { value: pwd } });
			await fireEvent.submit(form, { currentTarget: form });

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
			const { container } = render(Login, {});
			const form = getAsHTMLElement(container, "form");
			const textInput = getTextInput(container);

			await fireEvent.input(textInput, { target: { value: `  \t${pwd} \t  ` } });
			await fireEvent.submit(form, { currentTarget: form });
			await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

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
	});
});
