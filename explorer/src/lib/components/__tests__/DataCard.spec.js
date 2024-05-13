import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

import { DataCard } from "..";

describe("DataCard", () => {
  const baseProps = {
    data: null,
    error: null,
    headerButtonDetails: { action: () => {}, disabled: false, label: "Button" },
    loading: false,
    title: "Title",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `DataCard` in the loading state", () => {
    const loading = true;
    const { container } = render(DataCard, {
      ...baseOptions,
      props: { ...baseProps, loading },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the `DataCard` in the error state", () => {
    const error = new Error("error");
    const { container } = render(DataCard, {
      ...baseOptions,
      props: { ...baseProps, error },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the `DataCard` in the no data state", () => {
    const data = new Array(0);
    const { container } = render(DataCard, {
      ...baseOptions,
      props: { ...baseProps, data },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the `DataCard` in the data state", () => {
    const data = new Array(2);
    const renderWithSlots = renderWithSimpleContent(DataCard, {
      ...baseOptions,
      props: { ...baseProps, data },
    });

    expect(renderWithSlots.container.firstChild).toMatchSnapshot();
  });

  it("should render the `DataCard` in the data state when loading is true", () => {
    const data = new Array(2);
    const loading = true;
    const renderWithSlots = renderWithSimpleContent(DataCard, {
      ...baseOptions,
      props: { ...baseProps, data, loading },
    });

    expect(renderWithSlots.container.firstChild).toMatchSnapshot();
  });

  it("should pass the correct function to the button on click event", async () => {
    const onClickMock = vi.fn();
    const headerButtonDetails = {
      action: onClickMock,
      disabled: false,
      label: "Back",
    };
    const { getByRole } = render(DataCard, {
      ...baseOptions,
      props: { ...baseProps, headerButtonDetails },
    });

    await fireEvent.click(getByRole("button"));

    expect(onClickMock).toHaveBeenCalledTimes(1);
  });

  it("should render the `DataCard` with a disabled button", () => {
    const headerButtonDetails = {
      action: () => {},
      disabled: true,
      label: "Back",
    };
    const { container, getByRole } = render(DataCard, {
      ...baseOptions,
      props: { ...baseProps, headerButtonDetails },
    });

    expect(getByRole("button")).toBeDisabled();
    expect(container.firstChild).toMatchSnapshot();
  });
});
