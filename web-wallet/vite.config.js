// eslint-disable-next-line import/no-unresolved
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig, loadEnv } from "vite";
import basicSsl from "@vitejs/plugin-basic-ssl";
import { nodePolyfills } from "vite-plugin-node-polyfills";

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd());
	const commonPlugins = [
		sveltekit(),
		nodePolyfills({
			globals: { Buffer: true },
			include: ["buffer"]
		})
	];

	return {
		define: {
			"CONFIG": {
				LOCAL_STORAGE_APP_KEY: process.env.npm_package_name
			},
			"process.env": {
				CURRENT_NODE: env.VITE_CURRENT_NODE,
				CURRENT_PROVER_NODE: env.VITE_CURRENT_PROVER_NODE,
				LOCAL_NODE: env.VITE_LOCAL_NODE,
				LOCAL_PROVER_NODE: env.VITE_LOCAL_PROVER_NODE,
				MAINNET_NODE: env.VITE_MAINNET_NODE,
				MAINNET_PROVER_NODE: env.VITE_MAINNET_PROVER_NODE,
				RKYV_TREE_LEAF_SIZE: env.VITE_RKYV_TREE_LEAF_SIZE,
				STAKE_CONTRACT: env.VITE_STAKE_CONTRACT,
				TESTNET_NODE: env.VITE_TESTNET_NODE,
				TESTNET_PROVER_NODE: env.VITE_TESTNET_PROVER_NODE,
				TRANSFER_CONTRACT: env.VITE_TRANSFER_CONTRACT,
				VITE_CONTRACT_STAKE_DISABLED: env.VITE_CONTRACT_STAKE_DISABLED,
				VITE_CONTRACT_TRANSFER_DISABLED: env.VITE_CONTRACT_TRANSFER_DISABLED,
				VITE_GAS_LIMIT_DEFAULT: env.VITE_GAS_LIMIT_DEFAULT,
				VITE_GAS_LIMIT_LOWER: env.VITE_GAS_LIMIT_LOWER,
				VITE_GAS_LIMIT_UPPER: env.VITE_GAS_LIMIT_UPPER,
				VITE_GAS_PRICE_DEFAULT: env.VITE_GAS_PRICE_DEFAULT,
				VITE_GAS_PRICE_LOWER: env.VITE_GAS_PRICE_LOWER,
				VITE_GAS_PRICE_UPPER: env.VITE_GAS_PRICE_UPPER,
				VITE_GET_QUOTE_API_ENDPOINT: env.VITE_GET_QUOTE_API_ENDPOINT,
				VITE_STAKING_ENABLED: env.VITE_STAKING_ENABLED,
				VITE_TRANSFER_ENABLED: env.VITE_TRANSFER_ENABLED
			}
		},
		plugins: mode === "development" ? [basicSsl(), ...commonPlugins] : commonPlugins,
		server: {
			proxy: {
				"/rusk": {
					rewrite: path => path.replace(/^\/rusk/, ""),
					target: "http://localhost:8080/"
				}
			}
		},
		test: {
			/** @see https://github.com/vitest-dev/vitest/issues/2834 */
			alias: [{ find: /^svelte$/, replacement: "svelte/internal" }],
			coverage: {
				all: true,
				exclude: ["**/*.d.ts", "src/routes/components-showcase/**"],
				include: ["src/**"]
			},
			env: {
				CURRENT_NODE: "http://127.0.0.1:8080/",
				CURRENT_PROVER_NODE: "http://127.0.0.1:8080/",
				LOCAL_NODE: "http://127.0.0.1:8080/",
				LOCAL_PROVER_NODE: "http://127.0.0.1:8080/",
				MAINNET_NODE: "",
				MAINNET_PROVER_NODE: "",
				RKYV_TREE_LEAF_SIZE: "632",
				STAKE_CONTRACT: "0200000000000000000000000000000000000000000000000000000000000000",
				TRANSFER_CONTRACT: "0100000000000000000000000000000000000000000000000000000000000000",
				VITE_CONTRACT_STAKE_DISABLED: "false",
				VITE_CONTRACT_TRANSFER_DISABLED: "false",
				VITE_GAS_LIMIT_DEFAULT: "20000000",
				VITE_GAS_LIMIT_LOWER: "10000000",
				VITE_GAS_LIMIT_UPPER: "1000000000",
				VITE_GAS_PRICE_DEFAULT: "1",
				VITE_GAS_PRICE_LOWER: "1",
				VITE_GET_QUOTE_API_ENDPOINT: "https://api.dusk.network/v1/quote"
			},
			environment: "jsdom",
			include: ["src/**/*.{test,spec}.{js,ts}"],
			setupFiles: ["./vite-setup.js"]
		}
	};
});
