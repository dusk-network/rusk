import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { addCountAndUnique } from "$lib/chain-info";
import { WorldMap } from "..";
import { apiNodeLocations } from "$lib/mock-data";

describe("World Map", () => {
  afterEach(cleanup);

  it("renders the WorldMap component with nodes", () => {
    const props = {
      nodes: addCountAndUnique(apiNodeLocations),
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
