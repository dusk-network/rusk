import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { base } from "$app/paths";

import { AppAnchorButton } from "..";

describe("AppAnchorButton", () => {
  const baseProps = {
    className: "foo bar",
    href: "/setup",
    id: "some-id",
  };

  afterEach(cleanup);

  it("should render an `AnchorButton` with the base path prepended to the `href` attribute, if the `href` represents an absolute URL", () => {
    const { container, getByRole, rerender } = render(
      AppAnchorButton,
      baseProps
    );
    const anchorA = getByRole("link");

    expect(container.firstChild).toMatchSnapshot();
    expect(anchorA).toHaveAttribute("href", `${base}${baseProps.href}`);
    expect(anchorA).toHaveClass("foo bar");
    expect(anchorA).toHaveAttribute("id", baseProps.id);

    rerender({ ...baseProps, href: "/" });

    const anchorB = getByRole("link");

    expect(anchorB).toHaveAttribute("href", `${base}/`);
    expect(anchorB).toHaveClass("foo bar");
    expect(anchorB).toHaveAttribute("id", baseProps.id);
  });

  it("should leave the `AnchorButton` as it is if the `href` points to a relative path", () => {
    const { getByRole } = render(AppAnchorButton, {
      ...baseProps,
      href: "foo/bar",
    });

    expect(getByRole("link")).toHaveAttribute("href", "foo/bar");
  });

  it("should leave the `AnchorButton` as it is if the `href` points to an external URL", () => {
    const href = "http://example.com";
    const { getByRole } = render(AppAnchorButton, { ...baseProps, href });

    expect(getByRole("link")).toHaveAttribute("href", href);
  });

  it("should forward the `onclick` event to the `AnchorButton`", async () => {
    const handler = vi.fn();
    const { component, getByRole } = render(AppAnchorButton, {
      ...baseProps,
      href: "#",
    });

    component.$on("click", handler);

    await fireEvent.click(getByRole("link"));

    expect(handler).toHaveBeenCalledTimes(1);
  });
});
