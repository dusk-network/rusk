import { describe, expect, it } from "vitest";
import { cleanNumberString } from "../";

describe("calculateAdaptiveCharCount", () => {
  it("should return a valid string represantation of a number when the decimal separator is `,`", () => {
    expect(cleanNumberString("12153,566,68468,,,351", ",")).toBe(
      "12153,56668468351"
    );
  });

  it("should return a valid string represantation of a number when the decimal separator is `.`", () => {
    expect(cleanNumberString("100.00..549..6.548", ".")).toBe("100.005496548");
  });

  it("should return an empty string if an empty string is passed", () => {
    expect(cleanNumberString("", ".")).toBe("");
  });

  it("should return an empty string if a string with non valid characters is passed", () => {
    expect(cleanNumberString("asdsasd/*-,/?!@#$%^&*()_=+", ".")).toBe("");
  });

  it("should return a valid string represantation of a number if a string containing non valid characters is passed", () => {
    expect(
      cleanNumberString("1321651.0518asds592asd/*-,/?!@#$%^&*()_=+", ".")
    ).toBe("1321651.0518592");
  });

  it("should return a valid string represantation of a number if a string more than one leading zero", () => {
    expect(cleanNumberString("000.2165", ".")).toBe("0.2165");

    expect(cleanNumberString("0002165", ".")).toBe("2165");
  });
});
