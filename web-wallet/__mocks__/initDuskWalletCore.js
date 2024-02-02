import { readFile } from "node:fs/promises";

/**
 * @param {Record<string, any>} imports
 * @returns {Promise<WebAssembly.Instance>}
 */
const init = async imports =>
	readFile(require.resolve("@dusk-network/dusk-wallet-core/dusk_wallet_core_bg.wasm"))
		.then(buffer => WebAssembly.instantiate(buffer, imports))
		.then(({ instance }) => instance);

export default init;
