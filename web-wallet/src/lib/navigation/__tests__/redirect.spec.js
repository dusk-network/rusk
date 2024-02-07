import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import * as SvelteKit from "@sveltejs/kit";
import { base } from "$app/paths";

import { redirect } from "..";

describe("redirect", () => {
	const redirectSpy = vi.spyOn(SvelteKit, "redirect");

	afterEach(() => {
		redirectSpy.mockClear();
	});

	afterAll(() => {
		redirectSpy.mockRestore();
	});

	it("should add the defined base path to SvelteKit's `redirect` calls for absolute paths", () => {
		redirect(300, "/");
		redirect(301, "/foo/path");

		expect(redirectSpy).toHaveBeenCalledTimes(2);
		expect(redirectSpy).toHaveBeenNthCalledWith(1, 300, `${base}/`);
		expect(redirectSpy).toHaveBeenNthCalledWith(2, 301, `${base}/foo/path`);
	});

	it("should add nothing for relative paths and complete string URLs", async () => {
		redirect(300, "foo/bar");
		redirect(300, "http://example.com/");

		expect(redirectSpy).toHaveBeenCalledTimes(2);
		expect(redirectSpy).toHaveBeenNthCalledWith(1, 300, "foo/bar");
		expect(redirectSpy).toHaveBeenNthCalledWith(2, 300, "http://example.com/");
	});

	it("should do nothing if the received path is an URL object", async () => {
		const url = new URL("http://www.example.com/");

		redirect(300, url);

		expect(redirectSpy).toHaveBeenCalledTimes(1);
		expect(redirectSpy).toHaveBeenCalledWith(300, url);
	});
});
