import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import * as SvelteKit from "@sveltejs/kit";
import { Wallet } from "@dusk-network/dusk-wallet-js";

import { walletStore } from "$lib/stores";

import { load } from "../+layout";

describe("App layout.js", () => {
	const redirectSpy = vi.spyOn(SvelteKit, "redirect");

	afterEach(() => {
		redirectSpy.mockClear();
	});

	afterAll(() => {
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
