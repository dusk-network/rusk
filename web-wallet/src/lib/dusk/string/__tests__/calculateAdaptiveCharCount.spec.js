import { describe, expect, it } from "vitest";
import { calculateAdaptiveCharCount } from "../";

describe("calculateAdaptiveCharCount", () => {
	it("should return minimum characters for widths less than the minimum width", () => {
		expect(calculateAdaptiveCharCount(300)).toBe(5);
	});

	it("should return minimum characters for width equal to the minimum width", () => {
		expect(calculateAdaptiveCharCount(320, 320, 640, 5, 20)).toBe(5);
	});

	it("should return correct characters for a width between the minimum and maximum widths", () => {
		expect(calculateAdaptiveCharCount(500, 320, 640, 5, 20)).toBe(13);
	});

	it("should return maximum characters for width equal to the maximum width", () => {
		expect(calculateAdaptiveCharCount(800, 320, 640, 5, 20)).toBe(20);
	});

	it("should return maximum characters for widths greater than the maximum width", () => {
		expect(calculateAdaptiveCharCount(900, 320, 640, 5, 20)).toBe(20);
	});
});
