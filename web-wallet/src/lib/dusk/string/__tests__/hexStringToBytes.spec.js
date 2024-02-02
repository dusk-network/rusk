import {
	describe,
	expect,
	it
} from "vitest";

import { hexStringToBytes } from "..";

describe("hexStringToBytes", () => {
	it("should convert a hexadecimal string into a `Uint8Array`", () => {
		const expected = Uint8Array.of(255, 174, 2, 83);

		expect(hexStringToBytes("ffae0253")).toStrictEqual(expected);
		expect(hexStringToBytes("FFAE0253")).toStrictEqual(expected);
	});

	it("should convert invalid hex numbers to zeroes", () => {
		expect(hexStringToBytes("ffaeXX")).toStrictEqual(Uint8Array.of(255, 174, 0));
	});

	it("should return an empty `Uint8Array` if supplied with an empty string", () => {
		expect(hexStringToBytes("")).toStrictEqual(new Uint8Array());
	});
});
