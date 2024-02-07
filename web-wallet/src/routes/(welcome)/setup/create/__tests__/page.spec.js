import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { addresses } from "$lib/mock-data";
import Create from "../+page.svelte";
import { settingsStore, walletStore } from "$lib/stores";
import { setKey } from "lamb";
import { Wallet } from "@dusk-network/dusk-wallet-js";
import { getSeedFromMnemonic } from "$lib/wallet";
import * as navigation from "$lib/navigation";
import * as bip39 from "bip39";
import * as walletService from "$lib/services/wallet";
import * as shuffleArray from "$lib/dusk/array";
import { tick } from "svelte";
import loginInfoStorage from "$lib/services/loginInfoStorage";

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

describe("Create", async () => {
	const walletGetPsksSpy = vi.spyOn(Wallet.prototype, "getPsks").mockResolvedValue(addresses);
	const mnemonic = "cart dad sail wreck robot grit combine noble rap farm slide sad";
	const mnemonicShuffled = ["grit", "wreck", "cart", "dad", "rap",
		"sail", "robot", "combine", "noble", "slide", "sad", "farm"];
	const pwd = "passwordpassword";
	const seed = getSeedFromMnemonic(mnemonic);
	const userId = (await new Wallet(seed).getPsks())[0];
	const generateMnemonicSpy = vi.spyOn(bip39, "generateMnemonic").mockReturnValue(mnemonic);
	const shuffleArraySpy = vi.spyOn(shuffleArray, "shuffleArray").mockReturnValue(mnemonicShuffled);
	const getWalletSpy = vi.spyOn(walletService, "getWallet");
	const gotoSpy = vi.spyOn(navigation, "goto");
	const settingsResetSpy = vi.spyOn(settingsStore, "reset");
	const clearAndInitSpy = vi.spyOn(walletStore, "clearLocalDataAndInit");

	afterEach(async () => {
		cleanup();
		settingsStore.reset();
		walletGetPsksSpy.mockClear();
		generateMnemonicSpy.mockClear();
		shuffleArraySpy.mockClear();
		clearAndInitSpy.mockClear();
		getWalletSpy.mockClear();
		gotoSpy.mockClear();
		settingsResetSpy.mockClear();
	});

	afterAll(() => {
		walletGetPsksSpy.mockRestore();
		generateMnemonicSpy.mockRestore();
		shuffleArraySpy.mockRestore();
		clearAndInitSpy.mockRestore();
		getWalletSpy.mockRestore();
		gotoSpy.mockRestore();
		settingsResetSpy.mockRestore();
	});

	it("should render the Existing Wallet notice step of the Create flow if there is a userId saved in localStorage", () => {
		settingsStore.update(setKey("userId", userId));

		const { container } = render(Create);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the Terms of Service step of the Create flow if there is no userId saved in localStorage", () => {
		const { container } = render(Create);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the `Securely store your seed phrase!` agreement step after the ToS", async () => {
		const { container, getByRole } = render(Create);

		const mathRandomSpy = vi.spyOn(Math, "random").mockReturnValue(42);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		expect(container.firstChild).toMatchSnapshot();

		mathRandomSpy.mockRestore();
	});

	it("should not allow the user proceed unless both agreement checks are selected on the `Securely store your seed phrase!` step", async () => {
		const { getByRole, getAllByRole } = render(Create);

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
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));

		expect(container.firstChild).toMatchSnapshot();
	});

	it("correctly renders the Mnemonic Verification page", async () => {
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		expect(container.firstChild).toMatchSnapshot();
	});

	it("doesn't let the user proceed if they have entered mismatching Mnemonic", async () => {
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		const wordButtonsWrapper =
			container.getElementsByClassName("dusk-mnemonic__validate-actions-wrapper")[0];

		const wordButtons = Array.from(wordButtonsWrapper.children);

		wordButtons.forEach(async button => {
			await fireEvent.click(button);
		});

		await tick();

		expect(container.firstChild).toMatchSnapshot();

		expect(getByRole("button", { name: "Next" })).toBeDisabled();
	});

	it("lets the user proceed if they have entered a matching Mnemonic", async () => {
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		const mnemonicSplit = mnemonic.split(" ");

		mnemonicSplit.forEach(async word => {
			await fireEvent.click(getByRole("button", { name: word }));
		});

		await tick();

		expect(container.firstChild).toMatchSnapshot();

		expect(getByRole("button", { name: "Next" })).toBeEnabled();
	});

	it("ensures that the Undo button on the Mnemonic Validate step works as expected", async () => {
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		const mnemonicSplit = mnemonic.split(" ");

		mnemonicSplit.forEach(async word => {
			await fireEvent.click(getByRole("button", { name: word }));
		});

		await tick();

		await fireEvent.click(getByRole("button", { name: "Undo" }));

		expect(container.firstChild).toMatchSnapshot();

		expect(getByRole("button", { name: "Next" })).toBeDisabled();
	});

	it("ensures the Password step renders as expected", async () => {
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		const mnemonicSplit = mnemonic.split(" ");

		mnemonicSplit.forEach(async word => {
			await fireEvent.click(getByRole("button", { name: word }));
		});

		await tick();

		await fireEvent.click(getByRole("button", { name: "Next" }));

		// Password disabled
		expect(container.firstChild).toMatchSnapshot();

		await fireEvent.click(getByRole("switch"));

		// Password enabled
		expect(container.firstChild).toMatchSnapshot();
	});

	it("ensures the Swap To Native Dusk step renders as expected", async () => {
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		const mnemonicSplit = mnemonic.split(" ");

		mnemonicSplit.forEach(async word => {
			await fireEvent.click(getByRole("button", { name: word }));
		});

		await tick();

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		expect(container.firstChild).toMatchSnapshot();
	});

	it("ensures the All Done step renders as expected", async () => {
		const { container, getByRole, getAllByRole } = render(Create);

		await fireEvent.click(getByRole("button", { name: "Accept" }));

		await fireEvent.click(getAllByRole("checkbox")[0]);
		await fireEvent.click(getAllByRole("checkbox")[1]);

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		const mnemonicSplit = mnemonic.split(" ");

		mnemonicSplit.forEach(async word => {
			await fireEvent.click(getByRole("button", { name: word }));
		});

		await tick();

		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));
		await fireEvent.click(getByRole("button", { name: "Next" }));

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should initialize the wallet without setting a password", async () => {
		const { getByRole, getAllByRole } = render(Create);

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

		mnemonicSplit.forEach(async word => {
			await fireEvent.click(getByRole("button", { name: word }));
		});

		await tick();

		await fireEvent.click(getByRole("button", { name: "Next" }));

		// Set Password step
		await fireEvent.click(getByRole("button", { name: "Next" }));
		expect(loginInfoStorage.get()).toBeNull();

		// Swap ERC20 to Native Dusk step
		await fireEvent.click(getByRole("button", { name: "Next" }));

		// All Done step
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

	it("should initialize the wallet encrypted mnemonic saved in localStorage", async () => {
		const { getByPlaceholderText, getByRole, getAllByRole } = render(Create);

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

		mnemonicSplit.forEach(async word => {
			await fireEvent.click(getByRole("button", { name: word }));
		});

		await tick();

		await fireEvent.click(getByRole("button", { name: "Next" }));

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

		// Swap ERC20 to Native Dusk step
		await fireEvent.click(getByRole("button", { name: "Next" }));

		// All Done step
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
