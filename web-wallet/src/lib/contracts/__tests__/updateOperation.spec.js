import { describe, expect, it } from "vitest";
import { updateOperation } from "..";
import { get } from "svelte/store";
import { operationsStore } from "$lib/stores";

describe("updateOperation", () => {
  it("should set the current operation in the operationStore", () => {
    updateOperation("some operation");
    expect(get(operationsStore).currentOperation).toBe("some operation");
  });
});
