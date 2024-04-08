import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { base } from "$app/paths";

import { AppImage } from "..";

describe("AppImage", () => {
  const baseProps = {
    alt: "Some alternative text",
    className: "foo bar",
    height: "600",
    src: "/images/some-image.jpg",
    width: "800",
  };

  afterEach(cleanup);

  it("should render an HTML image forwarding all attributes but with the base path prepended to the `src` if it's an absolute URL", () => {
    const { container, getByRole, rerender } = render(AppImage, baseProps);
    const imgA = getByRole("img");

    expect(container.firstChild).toMatchSnapshot();
    expect(imgA).toHaveAttribute("alt", baseProps.alt);
    expect(imgA).toHaveClass("foo bar");
    expect(imgA).toHaveAttribute("height", baseProps.height);
    expect(imgA).toHaveAttribute("src", `${base}${baseProps.src}`);
    expect(imgA).toHaveAttribute("width", baseProps.width);

    rerender({ ...baseProps, className: "baz", src: "/" });

    const imgB = getByRole("img");

    expect(imgB).toHaveAttribute("alt", baseProps.alt);
    expect(imgB).toHaveClass("baz");
    expect(imgB).toHaveAttribute("height", baseProps.height);
    expect(imgB).toHaveAttribute("src", `${base}/`);
    expect(imgB).toHaveAttribute("width", baseProps.width);
  });

  it("shouldn't touch the src attribute if it represents a relative path", () => {
    const src = "images/some-image.jpg";
    const { getByRole } = render(AppImage, { ...baseProps, src });

    expect(getByRole("img")).toHaveAttribute("src", src);
  });

  it("shoudn't touch the `src` attribute if it points to an external URL", () => {
    const src = "http://example.com/some-image.jpg";
    const { getByRole } = render(AppImage, { ...baseProps, src });

    expect(getByRole("img")).toHaveAttribute("src", src);
  });
});
