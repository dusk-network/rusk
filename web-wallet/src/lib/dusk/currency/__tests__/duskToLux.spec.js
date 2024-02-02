import {
	describe,
	expect,
	it
} from "vitest";

import { duskToLux } from "..";

describe("duskToLux", () => {
	it("should convert an amount in Dusk to Lux", () => {
		expect(duskToLux(1)).toBe(1e9);
		expect(duskToLux(3_456_789.012)).toBe(3_456_789_012_000_000);
	});
});
