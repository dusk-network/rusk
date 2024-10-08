// eslint-disable-next-line import/no-unresolved
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig, loadEnv } from "vite";
import { execSync } from "child_process";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd());
  const buildDate = new Date().toISOString().substring(0, 10);
  const buildHash = execSync(
    "git log -1 --grep='explorer:' --format=format:'%h'"
  );
  const APP_VERSION = process.env.npm_package_version ?? "unknown";
  const APP_BUILD_INFO = `${buildHash.toString() || "unknown"} ${buildDate}`;
  const commonPlugins = [sveltekit()];

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
        VITE_BLOCKS_LIST_ENTRIES: env.VITE_BLOCKS_LIST_ENTRIES,
        VITE_CHAIN_INFO_ENTRIES: env.VITE_CHAIN_INFO_ENTRIES,
        VITE_MARKET_DATA_REFETCH_INTERVAL:
          env.VITE_MARKET_DATA_REFETCH_INTERVAL,
        VITE_NODE_URL: env.VITE_NODE_URL,
        VITE_REFETCH_INTERVAL: env.VITE_REFETCH_INTERVAL,
        VITE_RUSK_PATH: env.VITE_RUSK_PATH,
        VITE_STATS_REFETCH_INTERVAL: env.VITE_STATS_REFETCH_INTERVAL,
        VITE_TRANSACTIONS_LIST_ENTRIES: env.VITE_TRANSACTIONS_LIST_ENTRIES,
      },
    },
    plugins: commonPlugins,
    server: {
      proxy: {
        "/rusk": {
          rewrite: (path) => path.replace(/^\/rusk/, ""),
          target: "http://localhost:8080/",
        },
      },
    },
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
        VITE_API_ENDPOINT: "https://api.dusk.network/v1",
        VITE_BLOCKS_LIST_ENTRIES: "100",
        VITE_CHAIN_INFO_ENTRIES: "15",
        VITE_MARKET_DATA_REFETCH_INTERVAL: "120000",
        VITE_NODE_URL: "",
        VITE_REFETCH_INTERVAL: "1000",
        VITE_RUSK_PATH: "",
        VITE_STATS_REFETCH_INTERVAL: "1000",
        VITE_TRANSACTIONS_LIST_ENTRIES: "100",
      },
      environment: "jsdom",
      globalSetup: ["./vite-global-setup.js"],
      include: ["src/**/*.{test,spec}.{js,ts}"],
      setupFiles: ["./vite-setup.js"],
    },
  };
});
