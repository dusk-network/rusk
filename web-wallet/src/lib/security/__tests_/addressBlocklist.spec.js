import { describe, expect, it } from "vitest";

import {
  DEFAULT_BLOCKLISTED_ADDRESSES,
  getBlocklistedRecipient,
} from "../addressBlocklist";

describe("addressBlocklist", () => {
  it("should match the default blocklisted recipients", () => {
    for (const entry of DEFAULT_BLOCKLISTED_ADDRESSES) {
      expect(getBlocklistedRecipient(entry.address)).toMatchObject({
        address: entry.address,
      });
    }
  });
});
