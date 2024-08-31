import { writable } from "svelte/store";

/** @type OperationsStoreContent */
const initialState = { currentOperation: "" };

/** @type OperationsStore */
const store = writable(initialState);

export default store;
