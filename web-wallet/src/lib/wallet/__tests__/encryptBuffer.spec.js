import { describe, expect, it } from "vitest";
import { generateMnemonic } from "bip39";

import { encryptBuffer, getSeedFromMnemonic } from "..";

describe("encryptBuffer", () => {
  it("should be able to encrypt a buffer using the given password", async () => {
    const pwd = "some password";
    const buffer = getSeedFromMnemonic(generateMnemonic());
    const result = await encryptBuffer(buffer, pwd);

    expect(result).toMatchObject({
      data: expect.any(Uint8Array),
      iv: expect.any(Uint8Array),
      salt: expect.any(Uint8Array),
    });
    expect(result.data.toString()).not.toBe(buffer.toString());
    expect(result.iv.length).toBe(12);
    expect(result.salt.length).toBe(32);
  });
});
