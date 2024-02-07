import {
	afterAll,
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import {
	cleanup,
	fireEvent,
	render
} from "@testing-library/svelte";
import { Wallet } from "@dusk-network/dusk-wallet-js";

import * as navigation from "$lib/navigation";
import { addresses } from "$lib/mock-data";
import { walletStore } from "$lib/stores";

import { load } from "../+layout";
import Layout from "../+layout.svelte";

describe("App layout.js", () => {
	const getPsksSpy = vi.spyOn(Wallet.prototype, "getPsks").mockResolvedValue(addresses);
	const redirectSpy = vi.spyOn(navigation, "redirect");

	afterEach(() => {
		getPsksSpy.mockClear();
		redirectSpy.mockClear();
	});

	afterAll(() => {
		getPsksSpy.mockRestore();
		redirectSpy.mockRestore();
	});

	it("should check if a wallet is missing in the `walletStore` and redirect the user to the login page", async () => {
		// @ts-ignore
		await expect(load()).rejects.toThrow();

		expect(redirectSpy).toHaveBeenCalledTimes(1);
		expect(redirectSpy).toHaveBeenCalledWith(307, "/");
	});

	it("should do nothing otherwise", async () => {
		await walletStore.init(new Wallet([], 0, 0));

		// @ts-ignore
		await expect(load()).resolves.toBe(void 0);

		expect(redirectSpy).not.toHaveBeenCalled();
	});
});

describe("App layout.svelte", () => {
	const logoutSpy = vi.spyOn(navigation, "logout");
	const removeListenerSpy = vi.spyOn(window, "removeEventListener");
	const key = `${CONFIG.LOCAL_STORAGE_APP_KEY}-preferences`;
	const storage = {
		a: 1,
		b: { c: "some string" },
		userId: "user-1"
	};
	const eventData = {
		key,
		newValue: JSON.stringify({ ...storage, userId: "user-2" }),
		oldValue: JSON.stringify(storage),
		storageArea: localStorage
	};

	beforeEach(() => {
		removeListenerSpy.mockClear();
	});

	afterEach(() => {
		cleanup();
		logoutSpy.mockClear();
	});

	afterAll(() => {
		logoutSpy.mockRestore();
		removeListenerSpy.mockRestore();
	});

	it("should react to storage changes and logout the user if the `userId` changed", async () => {
		render(Layout);

		await fireEvent(window, new StorageEvent("storage", eventData));

		expect(removeListenerSpy).toHaveBeenCalledTimes(1);
		expect(removeListenerSpy).toHaveBeenNthCalledWith(1, "storage", expect.any(Function));
		expect(logoutSpy).toHaveBeenCalledTimes(1);
		expect(logoutSpy).toHaveBeenCalledWith(true);
	});

	it("should do nothing if the `userId` remained the same", async () => {
		const data = {
			...eventData,
			newValue: JSON.stringify({
				...storage,
				a: 5,
				b: "foo"
			})
		};

		render(Layout);

		await fireEvent(window, new StorageEvent("storage", data));

		expect(removeListenerSpy).not.toHaveBeenCalled();
		expect(logoutSpy).not.toHaveBeenCalled();
	});

	it("should do nothing if the storage change hasn't happened in the preferences", async () => {
		const data = {
			...eventData,
			key: "some-key",
			newValue: "\"{}\"",
			oldValue: null
		};

		render(Layout);

		await fireEvent(window, new StorageEvent("storage", data));

		expect(removeListenerSpy).not.toHaveBeenCalled();
		expect(logoutSpy).not.toHaveBeenCalled();
	});
});
