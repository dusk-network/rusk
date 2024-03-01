import { validateAddress } from "..";
import { describe, expect, it } from "vitest";

describe("validateAddress", () => {
  const validAddresses = [
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM",
    "4xwKPC9UMvketmoNkDvyaJufcTZWmNn8giB8xWTf3Qk8nFkRW81nTVwSdGPcbomzHThPuoXsdFzrzwiMar6BEfdw",
    "5kB6VBePF8eFhFVjLwM1xrEL6yGBm1uDsoWyRjdqDQ2nNz8nECAsRh3MZiM6uEo6WmukqyKzzCK9B5rcPTnjZQgt",
    "4LaS4bWzFQtvxZ7frUaXbfm3xsbnHHYwNkGnLqqpmWPYQeSfbAPy7N4Md8gk5gHn9f4wxNSNyFJuyxcnXPSWTRMd",
    "gMxrVEH5aW7XuQiXN2Pm2YRLHyCNmokmBb1VzjcmcQg7gzmxstPnozdt7SvvMKLP71BadPsa5jmoWFc2WzWDYPo",
  ];

  const invalidAddresses = [
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
    }
  });

  it("fails when supplied with an invalid address", () => {
    for (const address of invalidAddresses) {
      const result = validateAddress(address);

      expect(result.isValid).toBe(false);
    }
  });
});
