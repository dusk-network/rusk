import {
	describe,
	expect,
	it,
	vi
} from "vitest";
import { enumerables } from "lamb";
import { generateMnemonic } from "bip39";

import { getSeedFromMnemonic } from "$lib/wallet";

import { getWallet } from "..";

vi.unmock("@dusk-network/dusk-wallet-js");

describe("getWallet", () => {
	it("should get a Wallet instance using a seed", async () => {
		const mnemonic = generateMnemonic();
		const seed = getSeedFromMnemonic(mnemonic);
		const wallet = await getWallet(seed);

		expect(enumerables(wallet)).toMatchInlineSnapshot(`
			[
			  "wasm",
			  "seed",
			  "gasLimit",
			  "gasPrice",
			  "getBalance",
			  "getPsks",
			  "sync",
			  "transfer",
			  "stake",
			  "stakeInfo",
			  "unstake",
			  "withdrawReward",
			  "history",
			  "reset",
			]
		`);
	});
});
