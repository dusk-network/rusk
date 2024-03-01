import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { base } from "$app/paths";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

import { AppAnchor } from "..";

describe("AppAnchor", () => {
  const baseProps = {
    className: "foo bar",
    href: "/setup",
    id: "some-id",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render an `Anchor` with the base path prepended to the `href` attribute, if the `href` represents an absolute URL", () => {
    const renderA = renderWithSimpleContent(AppAnchor, baseOptions);
    const anchorA = renderA.getByRole("link");

    expect(renderA.container.firstChild).toMatchSnapshot();
    expect(anchorA).toHaveAttribute("href", `${base}${baseProps.href}`);
    expect(anchorA).toHaveClass("foo bar");
    expect(anchorA).toHaveAttribute("id", baseProps.id);

    cleanup();

    const renderB = renderWithSimpleContent(AppAnchor, {
      ...baseOptions,
      props: { ...baseProps, href: "/" },
    });
    const anchorB = renderB.getByRole("link");

    expect(anchorB).toHaveAttribute("href", `${base}/`);
    expect(anchorB).toHaveClass("foo bar");
    expect(anchorB).toHaveAttribute("id", baseProps.id);
  });

  it("should leave the `Anchor` as it is if the `href` points to a relative path", () => {
    const { getByRole } = renderWithSimpleContent(AppAnchor, {
      ...baseOptions,
      props: { ...baseProps, href: "foo/bar" },
    });

    expect(getByRole("link")).toHaveAttribute("href", "foo/bar");
  });

  it("should leave the `Anchor` as it is if the `href` points to an external URL", () => {
    const href = "http://example.com";
    const { getByRole } = renderWithSimpleContent(AppAnchor, {
      ...baseOptions,
      props: { ...baseProps, href },
    });

    expect(getByRole("link")).toHaveAttribute("href", href);
  });

  it("should forward the `onclick` event to the `Anchor`", async () => {
    const handler = vi.fn();
    const { component, getByRole } = render(AppAnchor, {
      ...baseProps,
      href: "#",
    });

    component.$on("click", handler);

    await fireEvent.click(getByRole("link"));

    expect(handler).toHaveBeenCalledTimes(1);
  });
});
