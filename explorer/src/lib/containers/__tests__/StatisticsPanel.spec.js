import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { StatisticsPanel } from "..";

describe("Statistics Panel", () => {
  afterEach(cleanup);

  it("renders the StatisticsPanel component", () => {
    const { container } = render(StatisticsPanel);

    expect(container.firstChild).toMatchSnapshot();
  });
});
