import { afterEach, describe, expect, it } from "vitest";
import { cleanup } from "@testing-library/svelte";
import { DetailList } from "..";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

describe("Detail List", () => {
  const baseProps = { className: "foo bar" };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("renders the Detail List component", () => {
    const render = renderWithSimpleContent(DetailList, baseOptions);

    expect(render.container.firstChild).toMatchSnapshot();
  });
});
