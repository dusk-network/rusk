import { getAddressInfo } from "..";
import { describe, expect, it } from "vitest";

describe("getAddressInfo", () => {
  const validShieldedAddresses = [
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM",
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnM",
  ];

  const validPublicAddresses = [
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

  // Shielded and public addresses for self-referential checks
  const shieldedAddress =
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM";
  const publicAddress =
    "zTsZq814KfWUAQujzjBchbMEvqA1FiKBUakMCtAc2zCa74h9YVz4a2roYwS7LHDHeBwS1aap4f3GYhQBrxroYgsBcE4FJdkUbvpSD5LVXY6JRXNgMXgk6ckTPJUFKoHybff";

  it("passes with valid shielded addresses and checks self-referential status", () => {
    for (const address of validShieldedAddresses) {
      const result = getAddressInfo(address, shieldedAddress, publicAddress);

      // Valid address should pass validation
      expect(result.isValid).toBe(true);

      // Type should be "address"
      expect(result.type).toBe("address");

      // Check if the address matches the shielded address
      expect(result.isSelfReferential).toBe(address === shieldedAddress);
    }
  });

  it("passes with valid public addresses and checks self-referential status", () => {
    for (const account of validPublicAddresses) {
      const result = getAddressInfo(account, shieldedAddress, publicAddress);

      // Valid account should pass validation
      expect(result.isValid).toBe(true);

      // Type should be "account"
      expect(result.type).toBe("account");

      // Check if the account matches the public address
      expect(result.isSelfReferential).toBe(account === publicAddress);
    }
  });

  it("fails when supplied with an invalid input", () => {
    for (const input of invalidInputs) {
      const result = getAddressInfo(input, shieldedAddress, publicAddress);

      // Invalid inputs should fail validation
      expect(result.isValid).toBe(false);

      // Self-referential check should not apply to invalid inputs
      expect(result.isSelfReferential).toBeUndefined();
    }
  });
});
