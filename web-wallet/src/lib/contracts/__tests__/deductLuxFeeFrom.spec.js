import {
	describe,
	expect,
	it
} from "vitest";

import { deductLuxFeeFrom } from "..";

describe("deductLuxFeeFrom", () => {
	it("should deduct a fee in Lux from a Dusk amount and return a value in Dusk", () => {
		expect(deductLuxFeeFrom(1000, 20000000)).toBe(999.98);
		expect(deductLuxFeeFrom(1000.456, 40000000)).toBe(1000.416);
		expect(deductLuxFeeFrom(1000.456, 4e15)).toBe(-3998999.544);

		// the simple subtraction would return 664144.7698459921 which has an extra decimal digit
		expect(deductLuxFeeFrom(664_144.809845992, 40000000)).toBe(664_144.769845992);
	});
});
