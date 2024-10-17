import { validateAddress } from "..";
import { describe, expect, it } from "vitest";

describe("validateAddress", () => {
  const validAddresses = [
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM",
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnM",
  ];

  const validAccounts = [
    "zTsZq814KfWUAQujzjBchbMEvqA1FiKBUakMCtAc2zCa74h9YVz4a2roYwS7LHDHeBwS1aap4f3GYhQBrxroYgsBcE4FJdkUbvpSD5LVXY6JRXNgMXgk6ckTPJUFKoHybff",
    "zTsZq814KfWUAQujzjBchbMEvqA1FiKBUakMCtAc2zCa74h9YVz4a2roYwS7LHDHeBwS1aap4f3GYhQBrxroYgsBcE4FJdkUbvpSD5LVXY6JRXNgMXgk6ckTPJUFKoHybf",
  ];

  const invalidInputs = [
    // Invalid Base58
    "InvalidKey12345",

    // Too short
    "4LaS4bWzFQtvxZ7frUaXbfm3xsbnHHYwNkGnLqqpmWPY",

    // Too long
    "5kB6VBePF8eFhFVjLwM1xrEL6yGBm1uDsoWyRjdqDQ2nNz8nECAsRh3MZiM6uEo6WmukqyKzzCK9B5rcPTnjZQgtXXXXXXXX",

    // Empty string
    "",

    // Contains an invalid character (!)
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnM!",

    // Contains an invalid character (_)
    "47jNTgAhzn9_CKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnM",

    // Contains an invalid character ( )
    "47jNTgAhzn9 CKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnM",

    // Contains an invalid character (0)
    "47jNTgAhzn0KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM",

    // Contains an invalid character (O)
    "47jNTgAhznOKCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM",

    // Contains an invalid character (l)
    "47jNTgAhznlKCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM",

    // Contains an invalid character (I)
    "47jNTgAhznIKCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM",
  ];

  it("passes when supplied with a valid address", () => {
    for (const address of validAddresses) {
      const result = validateAddress(address);

      expect(result.isValid).toBe(true);
      expect(result.type).toBe("address");
    }
  });

  it("passes when supplied with a valid account", () => {
    for (const account of validAccounts) {
      const result = validateAddress(account);

      expect(result.isValid).toBe(true);
      expect(result.type).toBe("account");
    }
  });

  it("fails when supplied with an invalid input", () => {
    for (const input of invalidInputs) {
      const result = validateAddress(input);

      expect(result.isValid).toBe(false);
    }
  });
});
