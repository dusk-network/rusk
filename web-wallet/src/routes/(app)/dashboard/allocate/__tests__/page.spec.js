import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { cleanup, render } from "@testing-library/svelte";
import { operationsStore } from "$lib/stores";
import Allocation from "../+page.svelte";

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {typeof import("$lib/stores")} */
  const original = await importOriginal();

  return {
    ...original,
    operationsStore: {
      ...original.operationsStore,
    },
  };
});

vi.useFakeTimers();

describe("Allocate", () => {
  afterEach(cleanup);
  afterAll(() => {
    vi.doUnmock("$lib/stores");
  });

  it("should render the allocation page", async () => {
    const { container } = render(Allocation);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should set the current operation to an empty string when destroyed", async () => {
    const { component } = render(Allocation);

    component.$destroy();

    expect(get(operationsStore)).toStrictEqual({
      currentOperation: "",
    });
  });
});
