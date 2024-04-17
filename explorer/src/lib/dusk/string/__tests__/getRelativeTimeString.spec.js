import { describe, expect, it } from "vitest";
import { getRelativeTimeString } from "../";

describe("getRelativeTimeString", () => {
  it("should return the correct relative time", () => {
    const date = new Date();
    expect(getRelativeTimeString(date, "long")).toBe("now");

    date.setDate(date.getDate() - 2);
    expect(getRelativeTimeString(date, "long")).toBe("2 days ago");

    date.setDate(date.getDate() - 7);
    expect(getRelativeTimeString(date, "long")).toBe("last week");

    date.setDate(date.getDate() - 7);
    expect(getRelativeTimeString(date, "long")).toBe("2 weeks ago");

    date.setDate(date.getDate() - 15);
    expect(getRelativeTimeString(date, "long")).toBe("last month");

    date.setDate(date.getDate() - 40);
    expect(getRelativeTimeString(date, "long")).toBe("2 months ago");

    date.setDate(date.getDate() - 300);
    expect(getRelativeTimeString(date, "long")).toBe("last year");

    date.setDate(date.getDate() - 365);
    expect(getRelativeTimeString(date, "long")).toBe("2 years ago");
  });
});
