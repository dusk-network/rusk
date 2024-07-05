import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { ToggleableCard } from "../Cards";

describe("IconHeadingCard", () => {
  afterEach(cleanup);

  it("renders the ToggleableCard component with a heading", () => {
    const { container } = render(ToggleableCard, {
      props: {
        heading: "My Card",
      },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the ToggleableCard component with a heading and an icon", () => {
    const { container } = render(ToggleableCard, {
      props: {
        heading: "My Card",
        iconPath: "M3,3H21V21H3V3M5,5V19H19V5H5Z",
      },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the ToggleableCard component with a toggle", () => {
    const { container } = render(ToggleableCard, {
      props: {
        heading: "My Card",
        iconPath: "M3,3H21V21H3V3M5,5V19H19V5H5Z",
        isToggled: true,
      },
    });

    expect(container.firstChild).toMatchSnapshot();
  });
});
