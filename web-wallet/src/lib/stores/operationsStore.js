import { writable } from "svelte/store";

/** @type {import("svelte/store").Writable<{ currentOperation: string }>} */
const count = writable({ currentOperation: "" });

export default count;
