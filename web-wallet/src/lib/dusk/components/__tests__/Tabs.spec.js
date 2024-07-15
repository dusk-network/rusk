import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { mdiHome } from "@mdi/js";
import { getAsHTMLElement } from "$lib/dusk/test-helpers";

import { Tabs } from "..";

describe("Tabs", () => {
  /**
   * `@juggle/resize-observer` uses this to get the dimensions of
   * the observed element and, by specs, the callback won't fire
   * on the `observe` call if the dimensions are both `0`.
   */
  // @ts-ignore we don't need to mock the whole CSS declaration
  const gcsSpy = vi.spyOn(window, "getComputedStyle").mockReturnValue({
    height: "320px",
    width: "320px",
  });
  const rafSpy = vi.spyOn(window, "requestAnimationFrame");
  const cafSpy = vi.spyOn(window, "cancelAnimationFrame");
  const scrollBySpy = vi.spyOn(HTMLUListElement.prototype, "scrollBy");
  const scrollIntoViewSpy = vi.spyOn(HTMLLIElement.prototype, "scrollIntoView");
  const scrollLeftSpy = vi
    .spyOn(HTMLUListElement.prototype, "scrollLeft", "get")
    .mockReturnValue(0);
  const scrollToSpy = vi.spyOn(HTMLUListElement.prototype, "scrollTo");
  const scrollWidthSpy = vi
    .spyOn(HTMLUListElement.prototype, "scrollWidth", "get")
    .mockReturnValue(640);
  const ulClientWidthSpy = vi
    .spyOn(HTMLUListElement.prototype, "clientWidth", "get")
    .mockReturnValue(320);

  // needed by `@juggle/resize-observer`
  const ulOffsetWidthSpy = vi
    .spyOn(HTMLUListElement.prototype, "offsetWidth", "get")
    .mockReturnValue(320);

  const items = [
    "Dashboard",
    "User Settings",
    "User Profile",
    "Notifications",
    "Direct Messaging",
    "Task Manager",
    "Event Calendar",
    "Analytics",
    "Team Management",
    "Help",
  ].map((v) => ({ id: v.toLowerCase().replace(/ /g, "-"), label: v }));

  /** @type {TabItem[]} */
  const itemsWithTextAndIcon = items.map((item, idx) => ({
    ...item,
    icon: { path: mdiHome, position: idx % 2 === 0 ? "before" : "after" },
  }));

  /** @type {TabItem[]} */
  const itemsWithIcon = itemsWithTextAndIcon.map(({ id, icon }) => ({
    icon,
    id,
  }));
  const itemsWithIdOnly = items.map(({ id }) => ({ id }));

  const baseProps = {
    items,
    selectedTab: "user-settings",
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  /** @param {import("svelte").ComponentProps<Tabs>} props */
  const renderTabs = async (props) => {
    const renderResult = render(Tabs, { ...baseOptions, props });

    /**
     * `@juggle/resize-observer` uses some scheduling, so we
     * need to wait for the first observe to fire.
     */
    await vi.waitUntil(() => rafSpy.mock.calls.length > 0);

    // clearing `requestAnimationFrame` calls made by `@juggle/resize-observer`
    rafSpy.mockClear();

    return renderResult;
  };

  afterEach(() => {
    cleanup();
    rafSpy.mockClear();
    cafSpy.mockClear();
    scrollBySpy.mockClear();
    scrollIntoViewSpy.mockClear();
    scrollLeftSpy.mockClear();
    scrollToSpy.mockClear();
    scrollWidthSpy.mockClear();
  });

  afterAll(() => {
    gcsSpy.mockRestore();
    rafSpy.mockRestore();
    cafSpy.mockRestore();
    scrollBySpy.mockRestore();
    scrollIntoViewSpy.mockRestore();
    scrollLeftSpy.mockRestore();
    scrollToSpy.mockRestore();
    scrollWidthSpy.mockRestore();
    ulClientWidthSpy.mockRestore();
    ulOffsetWidthSpy.mockRestore();
  });

  it('should render a "Tabs" component and reset its scroll status if no tab is selected', async () => {
    const { container } = await renderTabs({
      ...baseProps,
      selectedTab: undefined,
    });
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");

    expect(tabsList.scrollTo).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollTo).toHaveBeenCalledWith(0, 0);
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should scroll the selected tab into view if there's a selection", async () => {
    const { container } = await renderTabs(baseProps);
    const tab = getAsHTMLElement(
      container,
      `[data-tabid="${baseProps.selectedTab}"]`
    );

    expect(tab.scrollIntoView).toHaveBeenCalledTimes(1);
  });

  it("should be able to render tabs with icon and text", async () => {
    const { container } = await renderTabs({
      ...baseProps,
      items: itemsWithTextAndIcon,
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should be able to render tabs with icons only", async () => {
    const { container } = await renderTabs({
      ...baseProps,
      items: itemsWithIcon,
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should use the id as label if the tab hasn't one and is without icon", async () => {
    const { container } = await renderTabs({
      ...baseProps,
      items: itemsWithIdOnly,
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should observe the tab list resize on mounting and stop observing when unmounting", async () => {
    const observeSpy = vi.spyOn(ResizeObserver.prototype, "observe");
    const disconnectSpy = vi.spyOn(ResizeObserver.prototype, "disconnect");
    const { container, unmount } = await renderTabs(baseProps);
    const tabsList = container.querySelector(".dusk-tabs-list");

    expect(observeSpy).toHaveBeenCalledTimes(1);
    expect(observeSpy).toHaveBeenCalledWith(tabsList);

    unmount();

    expect(disconnectSpy).toHaveBeenCalledTimes(1);

    observeSpy.mockRestore();
    disconnectSpy.mockRestore();
  });

  it("should pass additional class names and attributes to the root element", async () => {
    const { container } = await renderTabs({
      ...baseProps,
      className: "foo bar",
      id: "some-id",
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should fire a change event when a tab is selected and it's not the current selection", async () => {
    const { component, getAllByRole } = await renderTabs(baseProps);
    const tabs = getAllByRole("tab");

    let expectedTab = tabs[0];

    expect.assertions(3);

    component.$on("change", (event) => {
      expect(event.detail).toBe(expectedTab.dataset.tabid);
    });

    // does nothing as it's currently selected
    await fireEvent.click(tabs[1]);

    await fireEvent.click(expectedTab);

    expectedTab = tabs[1];

    await fireEvent.keyDown(expectedTab, { key: "Enter" });

    expectedTab = tabs[2];

    await fireEvent.keyDown(expectedTab, { key: " " });

    // does nothing as neither space or Enter are pressed
    await fireEvent.keyDown(tabs[1], { key: "f" });
  });

  it("should scroll a tab into view when it gains focus", async () => {
    const { getAllByRole } = await renderTabs(baseProps);
    const tabs = getAllByRole("tab");

    scrollIntoViewSpy.mockClear();

    await fireEvent.focusIn(tabs[0]);

    expect(tabs[0].scrollIntoView).toHaveBeenCalledTimes(1);
  });

  it("should hide and disable the scroll buttons if there is enough horizontal space", async () => {
    scrollWidthSpy.mockReturnValueOnce(0);

    const { container } = await renderTabs(baseProps);
    const leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    const rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    expect(leftBtn.getAttribute("hidden")).toBe("true");
    expect(leftBtn.getAttribute("disabled")).toBe("");
    expect(rightBtn.getAttribute("hidden")).toBe("true");
    expect(rightBtn.getAttribute("disabled")).toBe("");
  });

  it("should show the scroll buttons when there isn't enough horizontal space and enable the appropriate ones", async () => {
    const { container } = await renderTabs(baseProps);
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");

    let leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    let rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    expect(leftBtn.getAttribute("hidden")).toBe("false");
    expect(leftBtn.getAttribute("disabled")).toBe("");
    expect(rightBtn.getAttribute("hidden")).toBe("false");
    expect(rightBtn.getAttribute("disabled")).toBeNull();

    await fireEvent.mouseDown(rightBtn, { buttons: 1 });

    expect(rafSpy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledWith(5, 0);

    scrollBySpy.mockClear();
    rafSpy.mockClear();

    await fireEvent.mouseUp(rightBtn);

    expect(cafSpy).toHaveBeenCalledTimes(1);

    scrollLeftSpy.mockReturnValueOnce(320);

    await fireEvent.scroll(tabsList);

    leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    expect(leftBtn.getAttribute("hidden")).toBe("false");
    expect(leftBtn.getAttribute("disabled")).toBeNull();
    expect(rightBtn.getAttribute("hidden")).toBe("false");
    expect(rightBtn.getAttribute("disabled")).toBe("");

    scrollBySpy.mockClear();
    rafSpy.mockClear();

    await fireEvent.mouseDown(leftBtn, { buttons: 1 });

    expect(rafSpy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledTimes(1);
    expect(tabsList.scrollBy).toHaveBeenCalledWith(-5, 0);

    await fireEvent.mouseUp(rightBtn);
  });

  it("should keep scrolling while the scroll button is pressed", async () => {
    vi.useFakeTimers();

    const { container } = render(Tabs, baseOptions);
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");
    const rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    await vi.advanceTimersToNextTimerAsync();

    expect(rightBtn.getAttribute("hidden")).toBe("false");
    expect(rightBtn.getAttribute("disabled")).toBeNull();

    await fireEvent.mouseDown(rightBtn, { buttons: 1 });

    const n = 10;

    for (let i = 0; i < n - 1; i++) {
      await vi.advanceTimersToNextTimerAsync();
    }

    expect(tabsList.scrollBy).toHaveBeenCalledTimes(n);

    for (let i = 1; i <= n; i++) {
      expect(tabsList.scrollBy).toHaveBeenNthCalledWith(n, 5, 0);
    }

    await fireEvent.mouseUp(rightBtn);

    vi.runAllTimers();
    vi.useRealTimers();
  });

  it("should ignore mouse down events if the primary button isn't the only one pressed", async () => {
    const { container } = await renderTabs(baseProps);
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");
    const leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    const rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );

    await fireEvent.mouseDown(leftBtn, { buttons: 2 });

    await fireEvent.mouseDown(leftBtn, { buttons: 3 });

    await fireEvent.mouseDown(rightBtn, { buttons: 2 });

    await fireEvent.mouseDown(rightBtn, { buttons: 3 });

    expect(rafSpy).not.toHaveBeenCalled();
    expect(tabsList.scrollBy).not.toHaveBeenCalled();
  });

  it("should bring the nearest tab into view on mouse clicks on scroll buttons", async () => {
    const { container } = await renderTabs(baseProps);
    const tabsList = getAsHTMLElement(container, ".dusk-tabs-list");
    const leftBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:first-of-type"
    );
    const rightBtn = getAsHTMLElement(
      container,
      ".dusk-tab-scroll-button:last-of-type"
    );
    const firstTab = getAsHTMLElement(container, "[role='tab']:first-of-type");
    const lastTab = getAsHTMLElement(container, "[role='tab']:last-of-type");

    const tabsListGetRectSpy = vi
      .spyOn(tabsList, "getBoundingClientRect")
      .mockReturnValue(DOMRect.fromRect({ width: tabsList.clientWidth, x: 0 }));
    const firstTabGetRectSpy = vi
      .spyOn(firstTab, "getBoundingClientRect")
      .mockReturnValue(DOMRect.fromRect({ width: 100, x: -100 }));
    const lastTabGetRectSpy = vi
      .spyOn(lastTab, "getBoundingClientRect")
      .mockReturnValue(
        DOMRect.fromRect({ width: 100, x: tabsList.clientWidth })
      );

    scrollIntoViewSpy.mockClear();

    await fireEvent.click(rightBtn);

    expect(lastTab.scrollIntoView).toHaveBeenCalledTimes(1);

    scrollIntoViewSpy.mockClear();

    await fireEvent.click(leftBtn);

    expect(firstTab.scrollIntoView).toHaveBeenCalledTimes(1);

    tabsListGetRectSpy.mockRestore();
    firstTabGetRectSpy.mockRestore();
    lastTabGetRectSpy.mockRestore();
  });
});
