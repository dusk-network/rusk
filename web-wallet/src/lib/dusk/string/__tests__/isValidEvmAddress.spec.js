import { describe, expect, it } from "vitest";
import isValidEvmAddress from "../isValidEvmAddress";

describe("isValidEvmAddress", () => {
  it("should return true for valid EVM addresses with 0x prefix", () => {
    expect(
      isValidEvmAddress("0x1234567890abcdef1234567890abcdef12345678")
    ).toBe(true);
    expect(
      isValidEvmAddress("0xAbCdEf1234567890AbCdEf1234567890AbCdEf12")
    ).toBe(true);
    expect(
      isValidEvmAddress("0x0000000000000000000000000000000000000000")
    ).toBe(true);
    expect(
      isValidEvmAddress("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
    ).toBe(true);
  });

  it("should return true for valid EVM addresses without 0x prefix", () => {
    expect(isValidEvmAddress("1234567890abcdef1234567890abcdef12345678")).toBe(
      true
    );
    expect(isValidEvmAddress("AbCdEf1234567890AbCdEf1234567890AbCdEf12")).toBe(
      true
    );
    expect(isValidEvmAddress("0000000000000000000000000000000000000000")).toBe(
      true
    );
    expect(isValidEvmAddress("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")).toBe(
      true
    );
  });

  it("should return false for invalid addresses", () => {
    // Too short
    expect(isValidEvmAddress("0x123")).toBe(false);
    expect(isValidEvmAddress("123")).toBe(false);

    // Too long
    expect(
      isValidEvmAddress("0x1234567890abcdef1234567890abcdef123456789")
    ).toBe(false);
    expect(isValidEvmAddress("1234567890abcdef1234567890abcdef123456789")).toBe(
      false
    );

    // Invalid characters
    expect(
      isValidEvmAddress("0x1234567890abcdef1234567890abcdef1234567g")
    ).toBe(false);
    expect(
      isValidEvmAddress("0x1234567890abcdef1234567890abcdef1234567!")
    ).toBe(false);
    expect(isValidEvmAddress("1234567890abcdef1234567890abcdef1234567z")).toBe(
      false
    );

    // Empty or null
    expect(isValidEvmAddress("")).toBe(false);
    // @ts-ignore
    expect(isValidEvmAddress(null)).toBe(false);
    // @ts-ignore
    expect(isValidEvmAddress(undefined)).toBe(false);

    // Wrong type
    // @ts-ignore
    expect(isValidEvmAddress(123)).toBe(false);
    // @ts-ignore
    expect(isValidEvmAddress({})).toBe(false);
    // @ts-ignore
    expect(isValidEvmAddress([])).toBe(false);
  });

  it("should handle edge cases", () => {
    // Only 0x prefix
    expect(isValidEvmAddress("0x")).toBe(false);

    // Spaces
    expect(
      isValidEvmAddress("0x1234567890abcdef1234567890abcdef12345678 ")
    ).toBe(false);
    expect(
      isValidEvmAddress(" 0x1234567890abcdef1234567890abcdef12345678")
    ).toBe(false);

    // Multiple 0x prefixes
    expect(
      isValidEvmAddress("0x0x1234567890abcdef1234567890abcdef12345678")
    ).toBe(false);
  });
});
