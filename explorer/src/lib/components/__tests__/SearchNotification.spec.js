import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { SearchNotification } from "../";

describe("Search Notification", () => {
  const baseProps = {
    data: {
      query: "some query text",
      res: [],
    },
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  const eventHandler = vi.fn();

  afterEach(() => {
    cleanup();
    eventHandler.mockClear();
  });

  it("renders the Search Notification component with the query string", () => {
    const { container } = render(SearchNotification, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the Search Notification component with a Error message", () => {
    const props = {
      ...baseProps,
      data: {
        query: "some query text",
        res: new Error("HTTP Request failed - failed"),
      },
    };
    const { container } = render(SearchNotification, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should dispatch a 'close' event when the button is clicked", async () => {
    const { component, getByRole } = render(SearchNotification, baseOptions);

    component.$on("close", eventHandler);

    const button = getByRole("button");
    await fireEvent.click(button);
    expect(eventHandler).toHaveBeenCalledOnce();
  });
});
