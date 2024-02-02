import {
	describe,
	expect,
	it
} from "vitest";

import { base64ToBytes } from "..";

describe("base64ToBytes", () => {
	const expected = Uint8Array.of(1, 1, 2, 3, 5, 8, 13, 21);

	it("should convert a Uint8Array to a base 64 string", () => {
		expect(base64ToBytes("AQECAwUIDRU=")).toStrictEqual(expected);
	});
});
