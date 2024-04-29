import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import * as appNavigation from "$app/navigation";
import { base } from "$app/paths";

import { goto } from "..";

describe("goto", () => {
  const gotoSpy = vi.spyOn(appNavigation, "goto");

  afterEach(() => {
    gotoSpy.mockClear();
  });

  afterAll(() => {
    gotoSpy.mockRestore();
  });

  it("should add the defined base path to Svelte's `goto` calls for absolute paths", async () => {
    await goto("/");
    await goto("/foo/path");

    expect(gotoSpy).toHaveBeenCalledTimes(2);
    expect(gotoSpy).toHaveBeenNthCalledWith(1, `${base}/`);
    expect(gotoSpy).toHaveBeenNthCalledWith(2, `${base}/foo/path`);
  });

  it("should add nothing for relative paths and complete string URLs", async () => {
    await goto("foo/bar");
    await goto("http://example.com/");

    expect(gotoSpy).toHaveBeenCalledTimes(2);
    expect(gotoSpy).toHaveBeenNthCalledWith(1, "foo/bar");
    expect(gotoSpy).toHaveBeenNthCalledWith(2, "http://example.com/");
  });

  it("should do nothing if the received path is an URL object", async () => {
    const url = new URL("http://www.example.com/");

    await goto(url);

    expect(gotoSpy).toHaveBeenCalledTimes(1);
    expect(gotoSpy).toHaveBeenCalledWith(url);
  });
});
