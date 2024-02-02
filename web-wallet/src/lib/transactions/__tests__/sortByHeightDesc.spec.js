import {
	describe,
	expect,
	it
} from "vitest";

import { transactions } from "$lib/mock-data";

import { sortByHeightDesc } from "..";

describe("sortByHeightDesc", () => {
	it("should sort the list of transaction by `block_height` desc", () => {
		const expected = transactions.slice().sort((a, b) => b.block_height - a.block_height);

		expect(sortByHeightDesc(transactions)).toStrictEqual(expected);
	});
});
