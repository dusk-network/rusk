import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { WorldMap } from "..";

describe("World Map", () => {
  afterEach(cleanup);

  it("renders the WorldMap component without nodes if Error is passed", () => {
    const props = {
      nodes: new Error("error"),
    };
    const { container } = render(WorldMap, props);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the WorldMap component with nodes", () => {
    const props = {
      nodes: [
        { lat: 31.2222, lon: 121.4581 },
        { lat: 52.352, lon: 4.9392 },
        { lat: 33.5939, lon: -112.303 },
      ],
    };
    const { container } = render(WorldMap, props);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the WorldMap component even if nodes is empty array", () => {
    const props = {
      nodes: [],
    };
    const { container } = render(WorldMap, props);

    expect(container.firstChild).toMatchSnapshot();
  });
});
