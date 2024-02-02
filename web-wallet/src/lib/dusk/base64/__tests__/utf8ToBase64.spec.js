import {
	describe,
	expect,
	it
} from "vitest";

import { utf8ToBase64 } from "..";

describe("utf8ToBase64", () => {
	const source = "ma che bella città 😃";
	const source2 = "a Ā 𐀀 文 🦄";

	it("should convert a Uint8Array to a base 64 string", () => {
		expect(utf8ToBase64(source)).toBe("bWEgY2hlIGJlbGxhIGNpdHTDoCDwn5iD");
		expect(utf8ToBase64(source2)).toBe("YSDEgCDwkICAIOaWhyDwn6aE");
	});
});
