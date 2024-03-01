import { afterAll, afterEach, describe, expect, it, vi } from "vitest";

import * as navigation from "$lib/navigation";

import { load } from "../+page";

describe("Main +page.js", () => {
  const redirectSpy = vi.spyOn(navigation, "redirect");

  afterEach(() => {
    redirectSpy.mockClear();
  });

  afterAll(() => {
    redirectSpy.mockRestore();
  });

  it("should redirect the user to the setup page", async () => {
    // @ts-ignore
    expect(async () => await load()).rejects.toThrow();

    expect(redirectSpy).toHaveBeenCalledTimes(1);
    expect(redirectSpy).toHaveBeenCalledWith(301, "/setup");
  });
});
