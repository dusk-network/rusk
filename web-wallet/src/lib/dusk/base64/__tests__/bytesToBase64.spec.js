import {
	describe,
	expect,
	it
} from "vitest";

import { bytesToBase64 } from "..";

describe("bytesToBase64", () => {
	const source = Uint8Array.of(1, 1, 2, 3, 5, 8, 13, 21);

	it("should convert a Uint8Array to a base 64 string", () => {
		expect(bytesToBase64(source)).toBe("AQECAwUIDRU=");
	});
});
