import { afterEach, describe, expect, it, vi } from "vitest";
import { writable } from "svelte/store";
import { cleanup, render } from "@testing-library/svelte";

import { ProgressBar } from "..";

vi.mock("svelte/motion", () => {
  return {
    tweened: vi.fn((initialValue) => {
      // Return a mock store that immediately sets the value for testing purposes
      let value = initialValue;
      const { set, subscribe } = writable(value);
      return {
        set: (/** @type {any} */ newValue) => {
          // Simulate the tweening effect by setting the value immediately
          set(newValue);
          value = newValue;
        },
        subscribe,
        update: (/** @type {(arg0: any) => any} */ fn) => set(fn(value)),
      };
    }),
  };
});

describe("ProgressBar", () => {
  afterEach(cleanup);

  it("renders the ProgressBar component with no current percentage set", () => {
    const { container } = render(ProgressBar);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the ProgressBar component with current percentage set as zero", () => {
    const { container } = render(ProgressBar, {
      props: { currentPercentage: 0 },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("re-renders the ProgressBar component when the current percentage property changes", async () => {
    const { container, rerender } = render(ProgressBar, {
      props: { currentPercentage: 0 },
    });

    expect(container.firstChild).toMatchSnapshot();

    await rerender({ currentPercentage: 50 });

    expect(container.firstChild).toMatchSnapshot();
  });
});
