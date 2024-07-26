import {
  afterAll,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { act, cleanup, render } from "@testing-library/svelte";
import { marketDataStore } from "$lib/stores";
import { StaleDataNotice } from "../";

const mockedReadableStore = await vi.hoisted(async () => {
  const { mockReadableStore } = await import("$lib/dusk/test-helpers");
  return mockReadableStore({
    data: null,
    error: null,
    isLoading: false,
    lastUpdate: null,
  });
});

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {typeof import("$lib/stores")} */
  const original = await importOriginal();

  return {
    ...original,
    marketDataStore: {
      ...mockedReadableStore,
      isDataStale: vi.fn(() => false),
    },
  };
});

describe("StaleDataNotice", () => {
  const initialState = structuredClone(
    mockedReadableStore.getMockedStoreValue()
  );

  beforeEach(() => {
    mockedReadableStore.setMockedStoreValue(initialState);
  });

  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/stores");
  });

  it("does not render the Stale Data notice if data is not stale", () => {
    mockedReadableStore.setMockedStoreValue({
      ...initialState,
      data: { data: "some data" },
      lastUpdate: new Date(),
    });

    const { container } = render(StaleDataNotice, { target: document.body });
    expect(container.childElementCount).toBe(0);
  });

  it("renders the Stale Data notice if data is stale", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2024, 4, 20));

    mockedReadableStore.setMockedStoreValue({
      ...initialState,
      data: { data: "some data" },
      lastUpdate: new Date(),
    });

    vi.mocked(marketDataStore.isDataStale).mockReturnValueOnce(true);

    const { container } = render(StaleDataNotice, { target: document.body });

    expect(container.firstChild).toMatchSnapshot();

    vi.useRealTimers();
  });

  it("should react to data changes", async () => {
    mockedReadableStore.setMockedStoreValue({
      ...initialState,
      lastUpdate: null,
    });

    const { container } = render(StaleDataNotice, { target: document.body });

    expect(container.childElementCount).toBe(0);

    vi.mocked(marketDataStore.isDataStale).mockReturnValueOnce(true);

    await act(() => {
      mockedReadableStore.setMockedStoreValue({
        ...initialState,
        lastUpdate: new Date(),
      });
    });

    expect(container.firstChild).toHaveClass("dusk-icon");
  });
});
