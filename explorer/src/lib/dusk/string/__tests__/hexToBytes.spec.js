import { describe, expect, it } from "vitest";
import { hexToBytes } from "..";

describe("hexToBytes", () => {
  it("should correctly convert a valid hex string to Uint8Array", () => {
    expect(hexToBytes("a1b2")).toEqual(new Uint8Array([161, 178]));
    expect(hexToBytes("0f1e")).toEqual(new Uint8Array([15, 30]));
    expect(hexToBytes("0011223344")).toEqual(
      new Uint8Array([0, 17, 34, 51, 68])
    );
  });

  it("should handle uppercase hex strings correctly", () => {
    expect(hexToBytes("A1B2")).toEqual(new Uint8Array([161, 178]));
    expect(hexToBytes("0F1E")).toEqual(new Uint8Array([15, 30]));
  });

  it("should throw an error for odd-length hex strings", () => {
    expect(() => hexToBytes("a1b")).toThrow(Error);
    expect(() => hexToBytes("123")).toThrow(Error);
  });

  it("should throw an error for non-hex characters", () => {
    expect(() => hexToBytes("zxy1")).toThrow(Error);
    expect(() => hexToBytes("0x1g")).toThrow(Error);
  });

  it("should throw an error for an empty string", () => {
    expect(() => hexToBytes("")).toThrow(Error);
  });
});
