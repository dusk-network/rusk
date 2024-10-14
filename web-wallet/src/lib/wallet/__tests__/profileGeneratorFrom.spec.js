import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

import { getSeedFromMnemonic } from "$lib/wallet";

import { profileGeneratorFrom } from "..";

vi.mock("$lib/vendor/w3sper.js/src/mod", async (importOriginal) => {
  /** @type {typeof import("$lib/vendor/w3sper.js/src/mod")} */
  const original = await importOriginal();

  return {
    ...original,
    ProfileGenerator: vi.fn(),
  };
});

describe("profileGeneratorFrom", () => {
  afterEach(() => {
    vi.mocked(ProfileGenerator).mockClear();
  });

  afterAll(() => {
    vi.doUnmock("$lib/vendor/w3sper.js/src/mod");
  });

  it("should create a `ProfileGenerator` instance from a seed", async () => {
    const ProfileGeneratorMock = vi.mocked(ProfileGenerator);
    const mnemonic =
      "cart dad sail wreck robot grit combine noble rap farm slide sad";
    const seed = getSeedFromMnemonic(mnemonic);

    profileGeneratorFrom(seed);

    const seederResult = ProfileGeneratorMock.mock.calls[0][0]();

    expect(ProfileGeneratorMock).toHaveBeenCalledTimes(1);
    expect(seederResult).toBeInstanceOf(Promise);
    await expect(seederResult).resolves.toBe(seed);
  });
});
