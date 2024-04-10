import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { Switch } from "..";

describe("Switch", () => {
  const baseProps = {};

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it('should render the "Switch" component with a default tab index of `0`', () => {
    const { container, rerender } = render(Switch, baseOptions);

    expect(container.firstChild).toMatchSnapshot();

    rerender({ ...baseProps, value: true });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should use the received tab index", () => {
    const props = {
      ...baseProps,
      tabindex: 5,
    };
    const { container } = render(Switch, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the component in a disabled status with a tabindex of `-1`", () => {
    const props = {
      ...baseProps,
      disabled: true,
      tabindex: 5,
    };
    const { container, rerender } = render(Switch, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();

    rerender({ ...props, value: true });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the root element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      id: "some-id",
    };
    const { container } = render(Switch, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  describe("Event handlers", () => {
    it("should dispatch a `change` event when the switch is clicked", async () => {
      const { component, getByRole } = render(Switch, baseOptions);
      const switchElement = getByRole("switch");
      const handler = vi.fn();

      component.$on("change", handler);

      await fireEvent.click(switchElement);
      await fireEvent.click(switchElement);

      expect(handler).toHaveBeenCalledTimes(2);
      expect(handler).toHaveBeenNthCalledWith(
        1,
        expect.objectContaining({ detail: true })
      );
      expect(handler).toHaveBeenNthCalledWith(
        2,
        expect.objectContaining({ detail: false })
      );
    });

    it("should dispatch a `change` event when the user presses space on the switch", async () => {
      const { component, getByRole } = render(Switch, baseOptions);
      const switchElement = getByRole("switch");
      const handler = vi.fn();

      component.$on("change", handler);

      await fireEvent.keyDown(switchElement, { key: " " });
      await fireEvent.keyDown(switchElement, { key: " " });

      expect(handler).toHaveBeenCalledTimes(2);
      expect(handler).toHaveBeenNthCalledWith(
        1,
        expect.objectContaining({ detail: true })
      );
      expect(handler).toHaveBeenNthCalledWith(
        2,
        expect.objectContaining({ detail: false })
      );
    });

    it("should not dispatch an event if the user presses another key", async () => {
      const { component, getByRole } = render(Switch, baseOptions);
      const switchElement = getByRole("switch");
      const handler = vi.fn();

      component.$on("change", handler);

      await fireEvent.keyDown(switchElement, { key: "Enter" });
      await fireEvent.keyDown(switchElement, { key: "a" });

      expect(handler).not.toHaveBeenCalled();
    });

    it("should not dispatch an event if the switch is disabled", async () => {
      const props = {
        ...baseProps,
        disabled: true,
      };
      const { component, getByRole } = render(Switch, {
        ...baseOptions,
        props,
      });
      const switchElement = getByRole("switch");
      const handler = vi.fn();

      component.$on("change", handler);

      await fireEvent.click(switchElement);
      await fireEvent.keyDown(switchElement, { key: " " });

      expect(handler).not.toHaveBeenCalled();
    });
  });
});
