import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { ProfileGenerator } from "@dusk/w3sper";

import { getSeedFromMnemonic } from "$lib/wallet";

import { profileGeneratorFrom } from "..";

vi.mock("@dusk/w3sper", async (importOriginal) => {
  /** @type {typeof import("@dusk/w3sper")} */
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
    const seedCopy = seed.slice();

    await profileGeneratorFrom(seed);

    const seederResult = await ProfileGeneratorMock.mock.calls[0][0]();

    expect(ProfileGeneratorMock).toHaveBeenCalledTimes(1);
    expect(seederResult.toString()).toBe(seed.toString());

    // ensures that the function doesn't mutate the seed
    expect(seed.toString()).toBe(seedCopy.toString());
  });
});
