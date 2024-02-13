import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";

import * as appNavigation from "$app/navigation";

import { walletStore } from "$lib/stores";

import { logout } from "..";

vi.mock("$lib/stores/walletStore");

describe("logout", () => {
	const gotoSpy = vi.spyOn(appNavigation, "goto");

	afterEach(() => {
		gotoSpy.mockClear();
		vi.mocked(walletStore.reset).mockClear();
	});

	afterAll(() => {
		gotoSpy.mockRestore();
		vi.doUnmock("$lib/stores/walletStore");
	});

	it("should reset the wallet store and redirect the user to the homepage, if the logout is not forced", async () => {
		await logout(false);

		expect(walletStore.reset).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledWith("/");
	});

	it("should add a `forcedLogout` querystring parameter if the logout is forced", async () => {
		await logout(true);

		expect(walletStore.reset).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledTimes(1);
		expect(gotoSpy).toHaveBeenCalledWith("/forced-logout");
	});
});
