import { describe, expect, it } from "vitest";

import { decryptBuffer, encryptBuffer, generateMnemonic } from "..";

describe("decryptBuffer", () => {
  const plaintext = new TextEncoder().encode(generateMnemonic());
  const pwd = "some password";

  it("should be able to decrypt the mnemonic phrase using the given password", async () => {
    const encryptInfo = await encryptBuffer(plaintext, pwd);
    const decrypted = await decryptBuffer(encryptInfo, pwd);

    expect(decrypted.toString()).toBe(plaintext.buffer.toString());
  });
});
