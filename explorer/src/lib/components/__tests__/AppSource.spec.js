import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { ownPairs } from "lamb";
import { base } from "$app/paths";

import { AppSource } from "..";

/** @type {(container: HTMLElement) => HTMLSourceElement} */
const getSourceElementIn = (container) =>
  /** @type {HTMLSourceElement} */ (container.querySelector("source"));

describe("AppSource", () => {
  const commonProps = {
    "data-foo": "bar",
    height: "600",
    type: "image/jpeg",
    width: "800",
  };

  afterEach(cleanup);

  describe("src attribute", () => {
    const baseProps = {
      ...commonProps,
      src: "/images/some-image.jpg",
    };

    it("should render a source element forwarding all attributes but with the base path prepended to the `src` if it's an absolute URL", () => {
      const { container, rerender } = render(AppSource, baseProps);
      const source = getSourceElementIn(container);

      expect(container.firstChild).toMatchSnapshot();

      ownPairs(commonProps).forEach(([key, value]) => {
        expect(source).toHaveAttribute(key, value);
      });

      expect(source).toHaveAttribute("src", `${base}${baseProps.src}`);

      rerender({ ...baseProps, src: "/" });

      expect(getSourceElementIn(container)).toHaveAttribute("src", `${base}/`);
    });

    it("shouldn't touch a `src` with a relative path", () => {
      const src = "images/some-image.jpg";
      const { container } = render(AppSource, { ...baseProps, src });

      expect(getSourceElementIn(container)).toHaveAttribute("src", src);
    });

    it("shoudn't touch a `src` with an external URL", () => {
      const src = "http://example.com/some-image.jpg";
      const { container } = render(AppSource, { ...baseProps, src });

      expect(getSourceElementIn(container)).toHaveAttribute("src", src);
    });
  });

  describe("srcset attribute", () => {
    it("should render a source element forwarding all attributes but with the base path prepended to absolute URLs in the `srcset`", () => {
      const props = {
        ...commonProps,
        srcset: "/images/some-image.jpg 1.5x",
      };
      const { container, rerender } = render(AppSource, props);
      const source = getSourceElementIn(container);

      expect(container.firstChild).toMatchSnapshot();

      ownPairs(commonProps).forEach(([key, value]) => {
        expect(source).toHaveAttribute(key, value);
      });

      expect(source).toHaveAttribute("srcset", `${base}${props.srcset}`);

      rerender({ ...props, srcset: "/images/some-image.jpg" });

      expect(getSourceElementIn(container)).toHaveAttribute(
        "srcset",
        `${base}/images/some-image.jpg`
      );

      rerender({ ...props, srcset: "/ 1.5x" });

      expect(getSourceElementIn(container)).toHaveAttribute(
        "srcset",
        `${base}/ 1.5x`
      );
    });

    it("should be able to handle multiple values in the srcset", () => {
      const props = {
        ...commonProps,
        srcset: "/images/some-image.jpg 1.5x, /foo.jpg, /bar/baz.jpg 300w",
      };
      const { container } = render(AppSource, props);

      expect(getSourceElementIn(container)).toHaveAttribute(
        "srcset",
        `${base}/images/some-image.jpg 1.5x, ${base}/foo.jpg, ${base}/bar/baz.jpg 300w`
      );
    });

    it("shouldn't touch elements with relative or absolute URLs in the srcset", () => {
      const props = {
        ...commonProps,
        srcset:
          "/images/some-image.jpg 1.5x, http://example.com/foo.jpg, bar/baz.jpg 300w",
      };
      const { container } = render(AppSource, props);

      expect(getSourceElementIn(container)).toHaveAttribute(
        "srcset",
        `${base}/images/some-image.jpg 1.5x, http://example.com/foo.jpg, bar/baz.jpg 300w`
      );
    });
  });
});
