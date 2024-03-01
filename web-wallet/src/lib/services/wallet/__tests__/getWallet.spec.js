import { describe, expect, it, vi } from "vitest";
import { enumerables } from "lamb";
import { generateMnemonic } from "bip39";

import { getSeedFromMnemonic } from "$lib/wallet";
import { getWallet } from "..";

vi.unmock("@dusk-network/dusk-wallet-js");

describe("getWallet", () => {
  it("should get a Wallet instance using a seed", () => {
    const mnemonic = generateMnemonic();
    const seed = getSeedFromMnemonic(mnemonic);
    const wallet = getWallet(seed);
    const walletPublicMembers = [
      ...enumerables(wallet),
      ...Object.getOwnPropertyNames(Object.getPrototypeOf(wallet)),
    ];

    expect(walletPublicMembers).toMatchInlineSnapshot(`
			[
			  "wasm",
			  "seed",
			  "gasLimit",
			  "gasPrice",
			  "constructor",
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
