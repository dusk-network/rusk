import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Table, TableBody, TableCell, TableHead, TableRow } from "../table";

describe("Table", () => {
  afterEach(cleanup);

  it("renders the Table component", () => {
    const { container } = render(Table);

    expect(container.querySelector(".table-container")).toBeTruthy();
    expect(container.querySelector(".table")).toBeTruthy();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the TableBody component", () => {
    const { container } = render(TableBody);

    expect(container.querySelector(".table__body")).toBeTruthy();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the TableCell component", () => {
    const { container } = render(TableCell);

    expect(container.querySelector(".table__data-cell")).toBeTruthy();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the TableCell component as a head cell", () => {
    const { container } = render(TableCell, { props: { type: "th" } });

    expect(container.querySelector(".table__header-cell")).toBeTruthy();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the TableHead component", () => {
    const { container } = render(TableHead);

    expect(container.querySelector(".table__head")).toBeTruthy();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the TableRow component", () => {
    const { container } = render(TableRow);

    expect(container.querySelector(".table__row")).toBeTruthy();

    expect(container.firstChild).toMatchSnapshot();
  });
});
