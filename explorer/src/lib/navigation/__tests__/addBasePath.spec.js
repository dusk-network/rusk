import { describe, expect, it } from "vitest";
import { base } from "$app/paths";

import { addBasePath } from "..";

describe("addBasePath", () => {
  it("should add the base path to the received path if the received one is an absolute path", () => {
    expect(addBasePath("/")).toBe(`${base}/`);
    expect(addBasePath("/some-path")).toBe(`${base}/some-path`);
  });

  it("should add nothing if the received path is a relative one or a complete string URL", () => {
    expect(addBasePath("foo/bar")).toBe("foo/bar");
    expect(addBasePath("http://example.com/")).toBe("http://example.com/");
  });

  it("should add nothing if the received path is a URL object", () => {
    const url = new URL("http://www.example.com/");

    expect(addBasePath(url)).toBe(url);
  });
});
