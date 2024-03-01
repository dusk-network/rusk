import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Agreement } from "..";

describe("Agreement", () => {
  const baseProps = {
    controlId: "test",
    label: "I agree",
    name: "test",
  };

  afterEach(cleanup);

  it("renders the Agreement component", () => {
    const { container } = render(Agreement, { props: { ...baseProps } });

    expect(container.firstChild).toMatchSnapshot();
  });

  // Rest of the functionality is covered under the Checkbox component tests
});
