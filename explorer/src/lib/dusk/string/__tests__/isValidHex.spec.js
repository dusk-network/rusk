import { describe, expect, it } from "vitest";
import { isValidHex } from "..";

describe("isValidHex", () => {
  it("should return true for a valid hex string", () => {
    expect(
      isValidHex(
        "0xce4d041b4cce091e9388391730956169691ca37496e063a6a4dbc3cfe75a889c"
      )
    ).toBeTruthy();
    expect(
      isValidHex(
        "30786365346430343162346363653039316539333838333931373330393536"
      )
    ).toBeTruthy();
    expect(isValidHex("a1b2")).toBe(true);
    expect(isValidHex("A1B2")).toBe(true);
  });

  it("should return false for strings with odd number of characters", () => {
    expect(isValidHex("a1b")).toBe(false);
    expect(isValidHex("0x1a2")).toBe(false);
  });

  it("should return false for strings with non-hex characters", () => {
    expect(
      isValidHex(
        "0xce4d041b4cc;./?-*|e091e938309jklmnoprst56169691ca3~!@#$%^&*()_+=7496e063a6ae75a889c"
      )
    ).toBeFalsy();
    expect(isValidHex("ghijklmnoprstyuxwz")).toBeFalsy();
    expect(
      isValidHex("307863653464;./?-*|393165393338~!@#$%^&*()_+=30393536")
    ).toBeFalsy();
  });

  it("should return false for invalid prefixes", () => {
    expect(isValidHex("0x-1a2b")).toBe(false);
    expect(isValidHex("-0y1a2b")).toBe(false);
  });

  it("should return false for empty string inputs", () => {
    expect(isValidHex("")).toBe(false);
  });

  it("should return false for strings with leading or trailing spaces", () => {
    expect(isValidHex(" 0x1a2b ")).toBe(false);
  });

  it("should return false for a string with only the '0x' prefix", () => {
    expect(isValidHex("0x")).toBe(false);
  });
});
