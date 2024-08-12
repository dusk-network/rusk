import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import RelativeTimeCustomContent from "./test-components/RelativeTimeCustomContent.svelte";

import { RelativeTime } from "..";

describe("RelativeTime", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20, 15, 25, 30));

  const baseProps = {
    date: new Date(2024, 4, 20, 15, 25, 10),
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the relative time of the given date and should not updated it by default", async () => {
    const { container } = render(RelativeTime, baseOptions);
    const textContent = container.firstChild?.textContent;

    expect(container.firstChild).toMatchSnapshot();
    expect(textContent).toMatchInlineSnapshot(`"20 seconds ago"`);

    await vi.advanceTimersByTimeAsync(10000);

    expect(container.firstChild?.textContent).toBe(textContent);
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      "data-baz": "baz",
    };
    const { container } = render(RelativeTime, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should update the relative time every second if the `autoRefresh` property is set to `true`", async () => {
    const props = { ...baseProps, autoRefresh: true };
    const { container } = render(RelativeTime, { ...baseOptions, props });

    expect(container.firstChild?.textContent).toMatchInlineSnapshot(
      `"30 seconds ago"`
    );

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.firstChild?.textContent).toMatchInlineSnapshot(
      `"31 seconds ago"`
    );

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.firstChild?.textContent).toMatchInlineSnapshot(
      `"32 seconds ago"`
    );
  });

  it("should allow to put custom content in the default slot", async () => {
    const { container } = render(RelativeTimeCustomContent, {
      ...baseOptions,
      props: { date: baseProps.date },
    });

    expect(container.firstChild).toMatchSnapshot();
    expect(container.firstChild?.textContent).toMatchInlineSnapshot(
      `"The relative time now is 32 seconds ago"`
    );

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.firstChild?.textContent).toMatchInlineSnapshot(
      `"The relative time now is 33 seconds ago"`
    );

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.firstChild?.textContent).toMatchInlineSnapshot(
      `"The relative time now is 34 seconds ago"`
    );
  });
});
