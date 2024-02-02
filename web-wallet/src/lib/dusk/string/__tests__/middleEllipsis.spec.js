import { describe, expect, it } from "vitest";

import { middleEllipsis } from "..";

describe("middleEllipsis", () => {
	it("should return the original text if text length is less than or equal to twice n", () => {
		expect(middleEllipsis("Hello", 5)).toEqual("Hello");
		expect(middleEllipsis("Hi", 2)).toEqual("Hi");
	});

	it("should return text with ellipsis in the middle for longer texts", () => {
		expect(middleEllipsis("HelloWorld", 3)).toEqual("Hel...rld");
		expect(middleEllipsis("abcdef", 2)).toEqual("ab...ef");
	});

	it("should handle edge cases gracefully", () => {
		expect(middleEllipsis("", 2)).toEqual("");
		expect(middleEllipsis("A", 0)).toEqual("...");
		expect(middleEllipsis("HelloWorld", 0)).toEqual("...");
	});
});
