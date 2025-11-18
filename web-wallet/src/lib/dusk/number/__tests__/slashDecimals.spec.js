import { describe, expect, it } from "vitest";
import slashDecimals from "../slashDecimals.js";

describe("slashDecimals", () => {
  it("should return the same string if there are no decimals", () => {
    expect(slashDecimals("42")).toBe("42");
  });

  it("should keep up to 9 decimal places when '.' is used", () => {
    expect(slashDecimals("123.1234567890")).toBe("123.123456789");
  });

  it("should keep up to 9 decimal places when ',' is used", () => {
    expect(slashDecimals("123,9876543210")).toBe("123,987654321");
  });

  it("should return the same number if it has fewer than 9 decimals", () => {
    expect(slashDecimals("1.123")).toBe("1.123");
  });

  it("should handle integers with a comma separator but no decimals", () => {
    expect(slashDecimals("100,")).toBe("100,");
  });

  it("should handle decimals shorter than 9 digits with trailing zeros", () => {
    expect(slashDecimals("0.1234000")).toBe("0.1234000");
  });

  it("should correctly process numbers that are too small", () => {
    expect(slashDecimals("0.0000000001234")).toBe("0.000000000");
  });

  it("should handle large numbers with more than 9 decimals", () => {
    expect(slashDecimals("987654321.123456789123")).toBe("987654321.123456789");
  });

  it("should handle comma decimals with less than 9 digits", () => {
    expect(slashDecimals("10,55")).toBe("10,55");
  });

  it("should not modify a string without separators", () => {
    expect(slashDecimals("5000")).toBe("5000");
  });
});
