import { operationsStore } from "$lib/stores";

/** @param {string} id */
function updateOperation(id) {
  operationsStore.update((store) => ({
    ...store,
    currentOperation: id,
  }));
}

export default updateOperation;
