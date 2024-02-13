import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";

import * as SvelteKit from "@sveltejs/kit";

import { load } from "../+page";

describe("Main +page.js", () => {
	const redirectSpy = vi.spyOn(SvelteKit, "redirect");

	afterEach(() => {
		redirectSpy.mockClear();
	});

	afterAll(() => {
		redirectSpy.mockRestore();
	});

	it("should redirect the user to the setup page", () => {
		// @ts-ignore
		expect(async () => await load({ url: new URL("http://example.com") })).rejects.toThrow();

		expect(redirectSpy).toHaveBeenCalledTimes(1);
		expect(redirectSpy).toHaveBeenCalledWith(301, "/setup");
	});
});
