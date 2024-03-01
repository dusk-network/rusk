import { afterAll, describe, expect, it, vi } from "vitest";
import { randomInt, sort } from "lamb";
import shuffleArray from "../shuffleArray.js";

vi.mock("lamb");

describe("shuffleArray", () => {
  const sampleArray = [1, 2, 3, 4, 5];

  afterAll(() => {
    vi.doUnmock("lamb");
  });

  it("should not mutate the original array", () => {
    const copyOfOriginal = [...sampleArray];

    shuffleArray(sampleArray);
    expect(sampleArray).toStrictEqual(copyOfOriginal);
  });

  it("should return an array of the same length", () => {
    const shuffledArray = shuffleArray(sampleArray);

    expect(shuffledArray.length).toBe(sampleArray.length);
  });

  it("should contain the same elements", () => {
    const shuffledArray = shuffleArray(sampleArray);

    expect(sort(shuffledArray)).toStrictEqual(sort(sampleArray));
  });

  it("should shuffle the array elements", () => {
    vi.mocked(randomInt)
      .mockReturnValue(2)
      .mockReturnValueOnce(3)
      .mockReturnValueOnce(4)
      .mockReturnValueOnce(0);

    let shuffledArray = shuffleArray(sampleArray);

    expect(shuffledArray).toStrictEqual([3, 1, 2, 4, 5]);

    shuffledArray = shuffleArray(sampleArray);
    expect(shuffledArray).toStrictEqual([1, 4, 2, 5, 3]);

    shuffledArray = shuffleArray(sampleArray);
    expect(shuffledArray).toStrictEqual([1, 4, 2, 5, 3]);
  });
});
