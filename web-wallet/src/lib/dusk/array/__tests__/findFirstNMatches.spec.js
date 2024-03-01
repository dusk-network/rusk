import { describe, expect, test } from "vitest";
import findFirstNMatches from "../findFirstNMatches";

describe("findFirstNMatches", () => {
  // Basic Functionality
  test("returns correct matches for standard input", () => {
    const words = ["apple", "apricot", "banana", "avocado", "apex"];
    const prefix = "ap";
    const numMatches = 3;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "apple",
      "apricot",
      "apex",
    ]);
  });

  // No Matches
  test("returns empty array when no matches found", () => {
    const words = ["apple", "banana", "cherry"];
    const prefix = "x";
    const numMatches = 2;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([]);
  });

  // Exact Number of Matches
  test("returns all matches when exact number of matches are requested", () => {
    const words = ["apple", "apricot", "appraisal"];
    const prefix = "ap";
    const numMatches = 3;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "apple",
      "apricot",
      "appraisal",
    ]);
  });

  // More Matches Requested than Available
  test("returns only available matches when more matches are requested", () => {
    const words = ["apple", "apricot"];
    const prefix = "ap";
    const numMatches = 5;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "apple",
      "apricot",
    ]);
  });

  // Case Sensitivity
  test("is case-sensitive", () => {
    const words = ["Apple", "apricot", "Apex"];
    const prefix = "Ap";
    const numMatches = 2;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "Apple",
      "Apex",
    ]);
  });

  // Empty Words Array
  test("returns empty array when words array is empty", () => {
    /**
     * @type {string[]}
     */
    const words = [];
    const prefix = "ap";
    const numMatches = 3;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([]);
  });

  // Empty Prefix
  test("returns matches for empty prefix", () => {
    const words = ["apple", "banana", "cherry"];
    const prefix = "";
    const numMatches = 2;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "apple",
      "banana",
    ]);
  });

  // Negative Number of Matches
  test("returns empty array for negative number of matches", () => {
    const words = ["apple", "banana", "cherry"];
    const prefix = "a";
    const numMatches = -1;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([]);
  });

  // Zero Number of Matches
  test("returns empty array when number of matches is zero", () => {
    const words = ["apple", "banana", "cherry"];
    const prefix = "a";
    const numMatches = 0;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([]);
  });

  // Large Dataset Performance
  test("handles large dataset efficiently", () => {
    const words = new Array(10000).fill("apple");

    words.push("apricot");

    const prefix = "ap";
    const numMatches = 1;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "apple",
    ]);
  });

  // Special Characters in Prefix and Words
  test("handles special characters in prefix and words", () => {
    const words = ["@pple", "#banana", "cherry!"];
    const prefix = "@";
    const numMatches = 1;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "@pple",
    ]);
  });

  // Prefix Longer than Word
  test("handles cases where prefix is longer than some words", () => {
    const words = ["app", "apple", "ap"];
    const prefix = "apple";
    const numMatches = 2;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "apple",
    ]);
  });

  // Words with Prefix in Middle or End
  test("only matches words starting with prefix", () => {
    const words = ["unhappy", "happy", "append", "end"];
    const prefix = "ap";
    const numMatches = 2;

    expect(findFirstNMatches(words, prefix, numMatches)).toStrictEqual([
      "append",
    ]);
  });
});
