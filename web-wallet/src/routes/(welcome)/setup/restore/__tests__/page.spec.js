import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { generateMnemonic } from "bip39";
import { Wallet } from "@dusk-network/dusk-wallet-js";
import { setKey } from "lamb";
import { get } from "svelte/store";
import { addresses } from "$lib/mock-data";
import * as navigation from "$lib/navigation";
import { settingsStore, walletStore } from "$lib/stores";
import { encryptMnemonic, getSeedFromMnemonic } from "$lib/wallet";
import loginInfoStorage from "$lib/services/loginInfoStorage";
import * as walletService from "$lib/services/wallet";
import { toastList } from "$lib/dusk/components/Toast/store";
import Restore from "../+page.svelte";
import { tick } from "svelte";

/**
 * @param {HTMLElement} input
 * @param {*} value
 * @returns {Promise<boolean>}
 */
const fireInput = (input, value) => fireEvent.input(input, { target: { value } });

/** @param {HTMLElement} element */
function asInput (element) {
	// eslint-disable-next-line no-extra-parens
	return /** @type {HTMLInputElement} */ (element);
}

describe("Restore", async () => {
	const walletGetPsksSpy = vi.spyOn(Wallet.prototype, "getPsks").mockResolvedValue(addresses);
	const mnemonic = generateMnemonic();
	const invalidMnemonic = "dad dad dad dad dad dad dad dad dad dad dad dad";
	const pwd = "passwordpassword";
	const loginInfo = await encryptMnemonic(mnemonic, pwd);
	const seed = getSeedFromMnemonic(mnemonic);
	const userId = (await new Wallet(seed).getPsks())[0];
	const getWalletSpy = vi.spyOn(walletService, "getWallet");
	const gotoSpy = vi.spyOn(navigation, "goto");
	const settingsResetSpy = vi.spyOn(settingsStore, "reset");
	const clearAndInitSpy = vi.spyOn(walletStore, "clearLocalDataAndInit");
	const readTextMock = vi.fn().mockResolvedValue(mnemonic);

	Object.assign(window.navigator, { clipboard: { readText: readTextMock } });

	afterEach(async () => {
		cleanup();
		clearAndInitSpy.mockClear();
		getWalletSpy.mockClear();
		gotoSpy.mockClear();
		settingsStore.reset();
		settingsResetSpy.mockClear();
		walletGetPsksSpy.mockClear();
		walletStore.reset();
		readTextMock.mockClear();
	});

	afterAll(() => {
		clearAndInitSpy.mockRestore();
		getWalletSpy.mockRestore();
		gotoSpy.mockRestore();
		settingsResetSpy.mockRestore();
		walletGetPsksSpy.mockRestore();
	});

	it("should render the Existing Wallet notice step of the Restore flow if there is a userId saved in localStorage", () => {
		settingsStore.update(setKey("userId", userId));

		const { container } = render(Restore);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the Terms of Service step of the Restore flow if there is no userId saved in localStorage", () => {
		const { container } = render(Restore);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the Mnemonic Authenticate step after accepting the Existing Wallet Notice and the Terms of Service", async () => {
		settingsStore.update(setKey("userId", userId));

		const { container, getByRole } = render(Restore);

		await fireEvent.click(getByRole("button", { name: "Proceed" }));
		await fireEvent.click(getByRole("button", { name: "Accept" }));

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should allow the user to proceed to password setup after a valid mnemonic has been provided", async () => {
		const { getByRole } = render(Restore);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		const nextButton = getByRole("button", { name: "Next" });

		expect(nextButton).toBeDisabled();

		await fireEvent.click(getByRole("button", { name: "Paste seed phrase" }));

		await tick();
		expect(nextButton).not.toBeDisabled();
	});

	it("should not allow the user to proceed to password setup after an invalid mnemonic has been provided", async () => {
		readTextMock.mockResolvedValueOnce(invalidMnemonic);

		const { getByRole } = render(Restore);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		const nextButton = getByRole("button", { name: "Next" });

		expect(nextButton).toBeDisabled();

		expect(get(toastList).length).toBe(0);

		await fireEvent.click(getByRole("button", { name: "Paste seed phrase" }));
		await tick();

		expect(get(toastList).length).toBe(1);
		expect(nextButton).toBeDisabled();
	});

	it("should initialize the wallet without setting a password", async () => {
		loginInfoStorage.set(loginInfo);

		const { getByRole } = render(Restore);

		// ToS step
		await fireEvent.click(getByRole("button", { name: "Accept" }));

		// Mnemonic Authenticate step
		const nextButton = getByRole("button", { name: "Next" });

		expect(nextButton).toBeDisabled();

		await fireEvent.click(getByRole("button", { name: "Paste seed phrase" }));
		await tick();
		expect(nextButton).toBeEnabled();
		await fireEvent.click(nextButton);

		// Set Password step
		await fireEvent.click(getByRole("button", { name: "Next" }));
		expect(loginInfoStorage.get()).toBeNull();

		// Restore Wallet step
		await fireEvent.click(getByRole("button", { name: "Next" }));

		await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

		expect(settingsResetSpy).toHaveBeenCalledTimes(1);
		expect(getWalletSpy).toHaveBeenCalledTimes(1);
		expect(getWalletSpy).toHaveBeenCalledWith(seed);
		expect(clearAndInitSpy).toHaveBeenCalledTimes(1);
		expect(clearAndInitSpy).toHaveBeenCalledWith(expect.any(Wallet));
		expect(gotoSpy).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
	});

	it("should initialize the wallet with the encrypted mnemonic saved in localStorage", async () => {
		const { getByRole, getByPlaceholderText } = render(Restore);

		// ToS step
		await fireEvent.click(getByRole("button", { name: "Accept" }));

		// Mnemonic Authenticate step
		const nextButton = getByRole("button", { name: "Next" });

		expect(nextButton).toBeDisabled();

		await fireEvent.click(getByRole("button", { name: "Paste seed phrase" }));
		await tick();
		expect(nextButton).toBeEnabled();
		await fireEvent.click(nextButton);

		// Set Password step
		expect(loginInfoStorage.get()).toBeNull();

		await fireEvent.click(getByRole("switch"));

		await fireInput(asInput(getByPlaceholderText("Set Password")), pwd);
		await fireInput(asInput(getByPlaceholderText("Confirm Password")), pwd);

		expect(loginInfoStorage.get()).toBeNull();

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await vi.waitFor(() => {
			expect(loginInfoStorage.get()).not.toBeNull();
		});

		// Restore Wallet step
		await fireEvent.click(getByRole("button", { name: "Next" }));
		await vi.waitUntil(() => gotoSpy.mock.calls.length > 0);

		expect(settingsResetSpy).toHaveBeenCalledTimes(1);
		expect(getWalletSpy).toHaveBeenCalledTimes(1);
		expect(getWalletSpy).toHaveBeenCalledWith(seed);
		expect(clearAndInitSpy).toHaveBeenCalledTimes(1);
		expect(clearAndInitSpy).toHaveBeenCalledWith(expect.any(Wallet));
		expect(gotoSpy).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledWith("/dashboard");
	});
});
