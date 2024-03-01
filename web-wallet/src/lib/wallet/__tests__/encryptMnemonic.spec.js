import { describe, expect, it } from "vitest";
import { generateMnemonic } from "bip39";

import { encryptMnemonic } from "..";

describe("encryptMnemonic", () => {
  const mnemonic = generateMnemonic();
  const pwd = "some password";

  it("should be able to encrypt the mnemonic phrase using the given password", async () => {
    const result = await encryptMnemonic(mnemonic, pwd);

    expect(result).toMatchObject({
      data: expect.any(Uint8Array),
      iv: expect.any(Uint8Array),
      salt: expect.any(Uint8Array),
    });
    expect(result.iv.length).toBe(12);
    expect(result.salt.length).toBe(32);
  });
});
