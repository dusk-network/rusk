import { writable } from "svelte/store";

/** @type {import('svelte/store').Writable<string[]>} */
const mnemonicPhraseResetStore = writable([]);

export default mnemonicPhraseResetStore;
