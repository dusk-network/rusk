import { describe, expect, it } from "vitest";

import { decryptMnemonic, encryptMnemonic, generateMnemonic } from "..";

describe("decryptMnemonic", () => {
  const mnemonic = generateMnemonic();
  const pwd = "some password";

  it("should be able to decrypt the mnemonic phrase using the given password", async () => {
    const mnemonicEncryptInfo = await encryptMnemonic(mnemonic, pwd);
    const decrypted = await decryptMnemonic(mnemonicEncryptInfo, pwd);

    expect(decrypted).toBe(mnemonic);
  });
});
