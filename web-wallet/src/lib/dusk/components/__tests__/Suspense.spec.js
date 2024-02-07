import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { rejectAfter, resolveAfter } from "$lib/dusk/test-helpers";

import { Suspense } from "..";

vi.useFakeTimers();

describe("Suspense", () => {
  const delay = 1000;

  const baseProps = {
    waitFor: resolveAfter(delay, "some result"),
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should be able to render the `Suspense` component in a pending state", () => {
    const { container } = render(Suspense, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      "data-baz": "baz",
      id: "some-id",
    };
    const { container } = render(Suspense, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should add appropriate class names for gap variants", () => {
    /** @type {import("svelte").ComponentProps<Suspense>} */
    const props = {
      ...baseProps,
      gap: "small",
    };
    const { container, rerender } = render(Suspense, { ...baseOptions, props });

    expect(container.firstChild).toHaveClass("dusk-suspense--small-gap");

    rerender({ ...props, gap: "large" });

    expect(container.firstChild).toHaveClass("dusk-suspense--large-gap");
  });

  it("should accept a custom message for the pending state", () => {
    const props = {
      ...baseProps,
      pendingMessage: "Operation pending",
    };
    const { container } = render(Suspense, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should be able to render the `Suspense` in a successful state", async () => {
    const { container } = render(Suspense, baseOptions);

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should be able to render the `Suspense` in a failure state with the "alert" variant as a default', async () => {
    const props = {
      ...baseProps,
      waitFor: rejectAfter(delay),
    };

    const { container } = render(Suspense, { ...baseOptions, props });

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should be able to render the `Suspense` in a failure state with the "details" error variant', async () => {
    /** @type {import("svelte").ComponentProps<Suspense>} */
    const props = {
      ...baseProps,
      errorVariant: "details",
      waitFor: rejectAfter(delay),
    };

    const { container } = render(Suspense, { ...baseOptions, props });

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a custom message for the failure state", async () => {
    const props = {
      ...baseProps,
      errorMessage: "Operation failed",
      waitFor: rejectAfter(delay),
    };

    const { container } = render(Suspense, { ...baseOptions, props });

    await vi.advanceTimersByTimeAsync(delay);

    expect(container.firstChild).toMatchSnapshot();
  });
});
