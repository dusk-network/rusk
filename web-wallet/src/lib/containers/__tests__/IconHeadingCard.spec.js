import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { IconHeadingCard } from "../Cards";

describe("IconHeadingCard", () => {
  afterEach(cleanup);

  it("renders the IconHeadingCard component with a heading", () => {
    const { container } = render(IconHeadingCard, {
      props: {
        heading: "My Card",
      },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the IconHeadingCard component with a heading and an icon", () => {
    const { container } = render(IconHeadingCard, {
      props: {
        heading: "My Card",
        iconPath: "M3,3H21V21H3V3M5,5V19H19V5H5Z",
      },
    });

    expect(container.firstChild).toMatchSnapshot();
  });
});
