// eslint-disable-next-line import/no-unresolved
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig, loadEnv } from "vite";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import { execSync } from "child_process";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd());
  const buildDate = new Date().toISOString().substring(0, 10);
  const buildHash = execSync(
    "git log -1 --grep='web-wallet:' --format=format:'%h'"
  );
  const APP_VERSION = process.env.npm_package_version ?? "unknown";
  const APP_BUILD_INFO = `${buildHash.toString() || "unknown"} ${buildDate}`;
  const commonPlugins = [
    sveltekit(),
    nodePolyfills({
      globals: { Buffer: true },
      include: ["buffer"],
    }),
  ];

  // needed to use %sveltekit.env.PUBLIC_APP_VERSION% in app.html
  process.env.PUBLIC_APP_VERSION = APP_VERSION;

  return {
    define: {
      CONFIG: {
        LOCAL_STORAGE_APP_KEY: process.env.npm_package_name,
      },
      "import.meta.env.APP_BUILD_INFO": JSON.stringify(APP_BUILD_INFO),
      "import.meta.env.APP_VERSION": JSON.stringify(APP_VERSION),
      "process.env": {
        API_ENDPOINT: env.VITE_API_ENDPOINT,
        VITE_DUSK_DEVNET_NODE: env.VITE_DUSK_DEVNET_NODE,
        VITE_DUSK_TESTNET_NODE: env.VITE_DUSK_TESTNET_NODE,
      },
    },
    plugins: commonPlugins,
    test: {
      /** @see https://github.com/vitest-dev/vitest/issues/2834 */
      alias: [{ find: /^svelte$/, replacement: "svelte/internal" }],
      coverage: {
        all: true,
        include: ["src/**"],
        provider: "istanbul",
      },
      env: {
        APP_BUILD_INFO: "hash1234 2024-01-12",
        APP_VERSION: "0.0.0",
        VITE_DUSK_DEVNET_NODE: "devnet.nodes.dusk.network",
        VITE_DUSK_TESTNET_NODE: "nodes.dusk.network",
      },
      environment: "jsdom",
      include: ["src/**/*.{test,spec}.{js,ts}"],
      setupFiles: ["./vite-setup.js"],
    },
  };
});
