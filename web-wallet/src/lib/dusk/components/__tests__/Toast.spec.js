import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";
import { fly } from "svelte/transition";
import { mdiAlertOutline } from "@mdi/js";

import { Toast } from "..";
import { toast, toastList } from "../Toast/store";

vi.mock("svelte/transition");

describe("Toast", () => {
  const baseProps = {
    flyDuration: 500,
    timer: 2000,
  };

  /** @param {HTMLElement} list */
  const getToastItems = (list) => list.querySelectorAll(".dusk-toast__item");

  vi.useFakeTimers();

  afterEach(() => {
    vi.mocked(fly).mockClear();
    cleanup();
  });

  afterAll(() => {
    vi.doUnmock("svelte/transition");
  });

  it("should render and dismiss the Toast component with the correct properties", async () => {
    const { getByRole } = render(Toast, baseProps);
    const list = getByRole("list");

    expect(get(toastList).length).toBe(0);
    expect(getToastItems(list).length).toBe(0);
    expect(fly).not.toHaveBeenCalled();

    toast("success", "Render Toast 1", mdiAlertOutline);

    await vi.advanceTimersToNextTimerAsync();

    const items = getToastItems(list);
    const toastStoredList = get(toastList);

    expect(toastStoredList.length).toBe(1);
    expect(toastStoredList[0]).toStrictEqual({
      icon: mdiAlertOutline,
      id: expect.any(String),
      message: "Render Toast 1",
      type: "success",
    });
    expect(items.length).toBe(1);
    expect(items[0]).toHaveTextContent("Render Toast 1");
    expect(
      items[0].querySelector(".dusk-toast__item-icon-wrapper--success")
    ).toBeDefined();
    expect(
      items[0].querySelector(`path[d="${mdiAlertOutline}"]`)
    ).toBeDefined();
    expect(fly).toHaveBeenCalledTimes(1);
    expect(fly).toHaveBeenCalledWith(
      items[0],
      expect.objectContaining({ duration: baseProps.flyDuration }),
      { direction: "in" }
    );
    expect(list).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(baseProps.timer);

    expect(get(toastList).length).toBe(0);
    expect(fly).toHaveBeenCalledTimes(2);
    expect(fly).toHaveBeenNthCalledWith(
      2,
      items[0],
      expect.objectContaining({ duration: baseProps.flyDuration }),
      { direction: "out" }
    );
  });

  it("should render the Toast component with the correct custom class and rest props", async () => {
    const customProps = {
      className: "test-toast",
      "data-baz": "baz",
      flyDuration: 500,
      id: "toast-container",
      timer: 2000,
    };

    const { getByRole } = render(Toast, customProps);
    const list = getByRole("list");

    expect(list).toHaveClass(`${customProps.className}`);
    expect(list).toHaveAttribute("id", customProps.id);
    expect(list).toHaveAttribute("data-baz", customProps["data-baz"]);

    expect(list).toMatchSnapshot();
  });

  it("should render and dismiss 2 Toasts", async () => {
    const { getByRole } = render(Toast, baseProps);
    const list = getByRole("list");

    expect(get(toastList).length).toBe(0);
    expect(getToastItems(list).length).toBe(0);
    expect(fly).not.toHaveBeenCalled();

    toast("success", "Render Toast 1", mdiAlertOutline);
    toast("info", "Render Toast 2", mdiAlertOutline);

    await vi.advanceTimersToNextTimerAsync();

    const items = getToastItems(list);
    const toastStoredList = get(toastList);

    expect(toastStoredList.length).toBe(2);
    expect(toastStoredList[0]).toStrictEqual({
      icon: mdiAlertOutline,
      id: expect.any(String),
      message: "Render Toast 1",
      type: "success",
    });
    expect(toastStoredList[1]).toStrictEqual({
      icon: mdiAlertOutline,
      id: expect.any(String),
      message: "Render Toast 2",
      type: "info",
    });
    expect(items.length).toBe(2);
    expect(items[0]).toHaveTextContent("Render Toast 1");
    expect(
      items[0].querySelector(".dusk-toast__item-icon-wrapper--success")
    ).toBeDefined();
    expect(
      items[0].querySelector(`path[d="${mdiAlertOutline}"]`)
    ).toBeDefined();

    expect(items[1]).toHaveTextContent("Render Toast 2");
    expect(
      items[1].querySelector(".dusk-toast__item-icon-wrapper--info")
    ).toBeDefined();
    expect(
      items[1].querySelector(`path[d="${mdiAlertOutline}"]`)
    ).toBeDefined();

    expect(fly).toHaveBeenCalledTimes(2);
    expect(fly).toHaveBeenCalledWith(
      items[0],
      expect.objectContaining({ duration: baseProps.flyDuration }),
      { direction: "in" }
    );
    expect(fly).toHaveBeenCalledWith(
      items[1],
      expect.objectContaining({ duration: baseProps.flyDuration }),
      { direction: "in" }
    );
    expect(list).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(baseProps.timer);

    expect(get(toastList).length).toBe(0);
    expect(fly).toHaveBeenCalledTimes(4);
    expect(fly).toHaveBeenNthCalledWith(
      3,
      items[0],
      expect.objectContaining({ duration: baseProps.flyDuration }),
      { direction: "out" }
    );
    expect(fly).toHaveBeenNthCalledWith(
      4,
      items[1],
      expect.objectContaining({ duration: baseProps.flyDuration }),
      { direction: "out" }
    );
  });
});
