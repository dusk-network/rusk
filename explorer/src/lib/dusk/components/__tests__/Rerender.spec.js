import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import RerenderCounter from "./test-components/RerenderCounter.svelte";
import RerenderGenerateValue1 from "./test-components/RerenderGenerateValue1.svelte";
import RerenderGenerateValue2 from "./test-components/RerenderGenerateValue2.svelte";

describe("renderWithSimpleContent", () => {
  vi.useFakeTimers();

  let domMutations = 0;

  const incrementMutations = () => domMutations++;
  const mutationObserver = new MutationObserver(incrementMutations);

  /**
   * @param {Parameters<render>[0]} component
   * @param {Parameters<render>[1]} options
   */
  const renderAndObserveContainer = (component, options) => {
    const renderResult = render(component, options);

    mutationObserver.observe(renderResult.container, {
      childList: true,
      subtree: true,
    });

    return renderResult;
  };

  const baseOptions = {
    target: document.body,
  };

  afterEach(() => {
    mutationObserver.disconnect();
    domMutations = 0;
    cleanup();
  });

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render its content and re-render it every second by default", async () => {
    const { container } = renderAndObserveContainer(
      RerenderCounter,
      baseOptions
    );

    expect(container.innerHTML).toMatchInlineSnapshot(`"0"`);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toMatchInlineSnapshot(`"1"`);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toMatchInlineSnapshot(`"2"`);

    expect(domMutations).toBe(2);
  });

  it("should accept a custom interval for re-renders", async () => {
    const props = { interval: 2000 };
    const { container } = renderAndObserveContainer(RerenderCounter, {
      ...baseOptions,
      props,
    });

    expect(container.innerHTML).toMatchInlineSnapshot(`"0"`);

    await vi.advanceTimersByTimeAsync(props.interval / 2);

    expect(container.innerHTML).toMatchInlineSnapshot(`"0"`);

    await vi.advanceTimersByTimeAsync(props.interval / 2);

    expect(container.innerHTML).toMatchInlineSnapshot(`"1"`);

    await vi.advanceTimersByTimeAsync(props.interval / 2);

    expect(container.innerHTML).toMatchInlineSnapshot(`"1"`);

    await vi.advanceTimersByTimeAsync(props.interval / 2);

    expect(container.innerHTML).toMatchInlineSnapshot(`"2"`);
    expect(domMutations).toBe(2);
  });

  it("should accept a custom `generateValue` function and use its result both as re-render key and as the default slot content", async () => {
    const values = [1, 2];
    const { container } = renderAndObserveContainer(RerenderGenerateValue1, {
      ...baseOptions,
      props: { values },
    });

    expect(container.innerHTML).toBe("1");

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("2");
    expect(domMutations).toBe(1);
  });

  it("should not trigger a re-render if the generated value is equal to the previous one by the `SameValueZero` comparison", async () => {
    const values = [1, 2, 0, -0, NaN, NaN, 3, 3, 4];
    const { container } = renderAndObserveContainer(RerenderGenerateValue1, {
      ...baseOptions,
      props: { values },
    });

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("2");

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("0");
    expect(domMutations).toBe(2);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("0");
    expect(domMutations).toBe(2);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("NaN");
    expect(domMutations).toBe(3);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("NaN");
    expect(domMutations).toBe(3);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("3");
    expect(domMutations).toBe(4);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("3");
    expect(domMutations).toBe(4);

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toBe("4");
    expect(domMutations).toBe(5);
  });

  it("should expose the custom generated value and let it be used as part of custom content", async () => {
    const { container } = renderAndObserveContainer(
      RerenderGenerateValue2,
      baseOptions
    );

    expect(container.innerHTML).toMatchInlineSnapshot(
      `"<span>now the value is: 0</span>"`
    );

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toMatchInlineSnapshot(
      `"<span>now the value is: 1</span>"`
    );

    await vi.advanceTimersByTimeAsync(1000);

    expect(container.innerHTML).toMatchInlineSnapshot(
      `"<span>now the value is: 2</span>"`
    );
    expect(domMutations).toBe(2);
  });
});
