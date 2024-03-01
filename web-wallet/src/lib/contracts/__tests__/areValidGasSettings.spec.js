import { describe, expect, it } from "vitest";
import { areValidGasSettings } from "..";
import { get } from "svelte/store";
import { gasStore } from "$lib/stores";

describe("areValidGasSettings", () => {
  const realStore = get(gasStore);

  it("should check to see if the store is read only", () => {
    // @ts-ignore
    expect(() => gasStore.set("value")).toThrowError();
  });

  it("should validate the provided gas limit and gas price based on the boundaries", () => {
    expect(
      areValidGasSettings(realStore.gasPriceLower, realStore.gasLimitUpper)
    ).toBe(true);

    expect(
      areValidGasSettings(realStore.gasPriceLower, realStore.gasLimitLower)
    ).toBe(true);

    expect(areValidGasSettings(realStore.gasPriceLower + 1, 0)).toBe(false);

    expect(
      areValidGasSettings(realStore.gasPriceLower, realStore.gasLimitLower - 1)
    ).toBe(false);

    expect(
      areValidGasSettings(
        realStore.gasPriceLower - 1,
        realStore.gasLimitUpper * 2
      )
    ).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings(NaN, realStore.gasLimitUpper)).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings(realStore.gasPriceLower, NaN)).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings(null, realStore.gasLimitUpper)).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings(realStore.gasPriceLower, null)).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings(undefined, realStore.gasLimitUpper)).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings(realStore.gasPriceLower, undefined)).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings("", realStore.gasLimitUpper)).toBe(false);

    // @ts-ignore
    expect(areValidGasSettings(realStore.gasPriceLower, "")).toBe(false);
  });
});
