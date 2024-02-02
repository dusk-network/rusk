import { describe, expect, it } from "vitest";

import { makeClassName } from "..";

describe("makeClassName", () => {
	it("should build a class name string from the keys holding \"truthy\" values if given an object as parameter", () => {
		const classes = {
			a: "",
			b: 0,
			c: -0,
			d: null,
			e: void 0,
			f: NaN,
			g: false,
			i: "false",
			j: "0",
			k: "true",
			l: true,
			m: ""
		};

		expect(makeClassName(classes)).toBe("i j k l");
	});

	it("should return an empty string if the object has no keys or all values are \"falsy\"", () => {
		const classes = {
			a: "", b: 0, c: -0, d: null, e: void 0, f: false, g: NaN
		};

		expect(makeClassName(classes)).toBe("");
		expect(makeClassName({})).toBe("");
	});

	it("should build a class name string from the unique \"truthy\" values if given an array as parameter", () => {
		const classes = ["", 0, -0, null, void 0, false, "false", "foo", "0", 2, "foo", "false"];

		expect(makeClassName(classes)).toBe("false foo 0 2");
	});

	it("should return an empty string if the array is empty or all values are \"falsy\"", () => {
		const classes = ["", 0, -0, null, void 0, false, NaN];

		expect(makeClassName(classes)).toBe("");
		expect(makeClassName([])).toBe("");
	});

	it("should throw an exception if the received parameter is `null` or `undefined`", () => {
		// @ts-expect-error
		expect(() => makeClassName(null)).toThrow();

		// @ts-expect-error
		expect(() => makeClassName(void 0)).toThrow();
	});
});
