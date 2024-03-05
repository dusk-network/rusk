import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { redirect as svelteKitRedirect } from "@sveltejs/kit";
import { base } from "$app/paths";

import { redirect } from "..";

vi.mock("@sveltejs/kit");

describe("redirect", () => {
  const redirectMock = vi.mocked(svelteKitRedirect);

  afterEach(() => {
    redirectMock.mockClear();
  });

  afterAll(() => {
    vi.doUnmock("@sveltejs/kit");
  });

  it("should add the defined base path to SvelteKit's `redirect` calls for absolute paths", () => {
    redirect(300, "/");
    redirect(301, "/foo/path");

    expect(redirectMock).toHaveBeenCalledTimes(2);
    expect(redirectMock).toHaveBeenNthCalledWith(1, 300, `${base}/`);
    expect(redirectMock).toHaveBeenNthCalledWith(2, 301, `${base}/foo/path`);
  });

  it("should add nothing for relative paths and complete string URLs", async () => {
    redirect(300, "foo/bar");
    redirect(300, "http://example.com/");

    expect(redirectMock).toHaveBeenCalledTimes(2);
    expect(redirectMock).toHaveBeenNthCalledWith(1, 300, "foo/bar");
    expect(redirectMock).toHaveBeenNthCalledWith(2, 300, "http://example.com/");
  });

  it("should do nothing if the received path is an URL object", async () => {
    const url = new URL("http://www.example.com/");

    redirect(300, url);

    expect(redirectMock).toHaveBeenCalledTimes(1);
    expect(redirectMock).toHaveBeenCalledWith(300, url);
  });
});
