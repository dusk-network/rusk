import {
	afterAll,
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { act, cleanup, fireEvent, render } from "@testing-library/svelte";
import * as appNavigation from "$app/navigation";

import mockedWalletStore from "../../__mocks__/mockedWalletStore";
import { settingsStore, walletStore } from "$lib/stores";

import Settings from "../+page.svelte";

vi.mock("$lib/stores", async importOriginal => {
	/** @type {import("$lib/stores/stores").WalletStore} */
	const original = await importOriginal();

	return {
		...original,
		walletStore: {
			// @ts-ignore
			...(await vi.importMock("$lib/stores/walletStore")).default,
			...mockedWalletStore
		}
	};
});

vi.useFakeTimers();

describe("Settings", () => {
	const initialWalletStoreState = structuredClone(mockedWalletStore.getMockedStoreValue());
	const gotoSpy = vi.spyOn(appNavigation, "goto");
	const resetSpy = vi.spyOn(walletStore, "reset");

	beforeEach(() => {
		mockedWalletStore.setMockedStoreValue(initialWalletStoreState);
	});

	afterEach(() => {
		cleanup();
		gotoSpy.mockClear();
		resetSpy.mockClear();
	});

	afterAll(() => {
		gotoSpy.mockRestore();
		resetSpy.mockRestore();
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
				isSyncing: true
			});
		});

		expect(resetButton).toHaveAttribute("disabled");
		expect(resetButton).toHaveAttribute("data-tooltip-disabled", "false");
	});

	it("should reset wallet store and navigate to login page on clicking the Log Out button", async () => {
		const { getByRole } = render(Settings);

		const button = getByRole("button", { name: "Log out" });

		await fireEvent.click(button);
		await vi.advanceTimersToNextTimerAsync();

		expect(resetSpy).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledWith("/");
	});

	describe("Resetting the wallet", () => {
		const clearDataSpy = vi.spyOn(walletStore, "clearLocalData").mockResolvedValue(void 0);
		const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
		const settingsResetSpy = vi.spyOn(settingsStore, "reset");

		afterEach(() => {
			clearDataSpy.mockClear();
			confirmSpy.mockClear();
			settingsResetSpy.mockClear();
		});

		afterAll(() => {
			clearDataSpy.mockRestore();
			confirmSpy.mockRestore();
			settingsResetSpy.mockRestore();
		});

		it("should clear local data and settings and then log out the user if the reset button is clicked and the user confirms the operation", async () => {
			const { getByRole } = render(Settings);
			const resetButton = getByRole("button", { name: /reset/i });

			await fireEvent.click(resetButton);

			expect(confirmSpy).toHaveBeenCalledTimes(1);
			expect(clearDataSpy).toHaveBeenCalledTimes(1);

			await vi.advanceTimersToNextTimerAsync();

			expect(settingsResetSpy).toHaveBeenCalledTimes(1);
			expect(resetSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledTimes(1);
			expect(gotoSpy).toHaveBeenCalledWith("/");
		});

		it("should do nothing if the user doesn't confirm the reset", async () => {
			confirmSpy.mockReturnValueOnce(false);

			const { getByRole } = render(Settings);
			const resetButton = getByRole("button", { name: /reset/i });

			await fireEvent.click(resetButton);

			expect(confirmSpy).toHaveBeenCalledTimes(1);
			expect(clearDataSpy).not.toHaveBeenCalled();

			await vi.advanceTimersToNextTimerAsync();

			expect(settingsResetSpy).not.toHaveBeenCalled();
			expect(resetSpy).not.toHaveBeenCalled();
			expect(gotoSpy).not.toHaveBeenCalled();
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
			expect(resetSpy).not.toHaveBeenCalled();
			expect(gotoSpy).not.toHaveBeenCalled();
			expect(container.firstChild).toMatchSnapshot();
		});
	});
});
