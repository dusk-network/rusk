import { sveltekit } from "@sveltejs/kit/vite";
import { coverageConfigDefaults } from "vitest/config";
import { default as basicSsl } from "@vitejs/plugin-basic-ssl";
import { defineConfig, loadEnv } from "vite";
import { execSync } from "child_process";
import { resolve } from "path";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd());
  const buildDate = new Date().toISOString().substring(0, 10);
  const buildHash = execSync(
    "git log -1 --grep='web-wallet:' --format=format:'%h'"
  );
  const APP_VERSION = process.env.npm_package_version ?? "unknown";
  const APP_BUILD_INFO = `${buildHash.toString() || "unknown"} ${buildDate}`;
  const commonPlugins = [sveltekit()];

  // needed to use %sveltekit.env.PUBLIC_APP_VERSION% in app.html
  process.env.PUBLIC_APP_VERSION = APP_VERSION;

  return {
    define: {
      CONFIG: { LOCAL_STORAGE_APP_KEY: process.env.npm_package_name },
      "import.meta.env.APP_BUILD_INFO": JSON.stringify(APP_BUILD_INFO),
      "import.meta.env.APP_VERSION": JSON.stringify(APP_VERSION),
      "process.env": {
        MODE_MAINTENANCE: env.VITE_MODE_MAINTENANCE,
        VITE_FEATURE_ALLOCATE: env.VITE_FEATURE_ALLOCATE,
        VITE_FEATURE_BRIDGE: env.VITE_FEATURE_BRIDGE,
        VITE_FEATURE_MIGRATE: env.VITE_FEATURE_MIGRATE,
        VITE_FEATURE_STAKE: env.VITE_FEATURE_STAKE,
        VITE_FEATURE_TRANSACTION_HISTORY: env.VITE_FEATURE_TRANSACTION_HISTORY,
        VITE_FEATURE_TRANSFER: env.VITE_FEATURE_TRANSFER,
        VITE_GAS_LIMIT_DEFAULT: env.VITE_GAS_LIMIT_DEFAULT,
        VITE_GAS_LIMIT_LOWER: env.VITE_GAS_LIMIT_LOWER,
        VITE_GAS_LIMIT_UPPER: env.VITE_GAS_LIMIT_UPPER,
        VITE_GAS_PRICE_DEFAULT: env.VITE_GAS_PRICE_DEFAULT,
        VITE_GAS_PRICE_LOWER: env.VITE_GAS_PRICE_LOWER,
        VITE_GAS_PRICE_UPPER: env.VITE_GAS_PRICE_UPPER,
        VITE_NODE_URL: env.VITE_NODE_URL,
        VITE_REOWN_PROJECT_ID: env.VITE_REOWN_PROJECT_ID,
        VITE_SYNC_INTERVAL: env.VITE_SYNC_INTERVAL,
      },
    },
    plugins:
      mode === "development" ? [basicSsl(), ...commonPlugins] : commonPlugins,
    server: {
      proxy: {
        "/on": { target: "ws://localhost:8080/", ws: true },
        "/rusk": {
          rewrite: (path) => path.replace(/^\/rusk/, ""),
          target: "http://localhost:8080/",
        },
        "/static/drivers": { target: "http://localhost:8080/" },
      },
    },
    test: {
      alias: [
        /** @see https://github.com/vitest-dev/vitest/issues/2834 */
        { find: /^svelte$/, replacement: "svelte/internal" },

        // Aliases to mock private w3sper's modules
        {
          find: /.+\/protocol-driver\/mod\.js$/,
          replacement: resolve("./src/lib/mocks/ProtocolDriver.js"),
        },
        {
          find: /.*\/components\/transactions\.js$/,
          replacement: resolve("./src/lib/mocks/Transactions.js"),
        },
      ],
      coverage: {
        all: true,
        exclude: [
          "src/routes/components-showcase/**",
          "src/lib/vendor/**",
          ...coverageConfigDefaults.exclude,
        ],
        include: ["src/**"],
        provider: "istanbul",
      },
      env: {
        APP_BUILD_INFO: "hash1234 2024-01-12",
        APP_VERSION: "0.1.5",
        VITE_FEATURE_ALLOCATE: "true",
        VITE_FEATURE_BRIDGE: "true",
        VITE_FEATURE_MIGRATE: "true",
        VITE_FEATURE_STAKE: "true",
        VITE_FEATURE_TRANSACTION_HISTORY: "true",
        VITE_FEATURE_TRANSFER: "true",
        VITE_GAS_LIMIT_DEFAULT: "100000000",
        VITE_GAS_LIMIT_LOWER: "10000000",
        VITE_GAS_LIMIT_UPPER: "1000000000",
        VITE_GAS_PRICE_DEFAULT: "1",
        VITE_GAS_PRICE_LOWER: "1",
        VITE_NODE_URL: "",
        VITE_SYNC_INTERVAL: "300000",
      },
      environment: "jsdom",
      include: ["src/**/*.{test,spec}.{js,ts}"],
      server: {
        deps: {
          // we inline w3sper and duskit to be able to use aliases
          inline: ["@dusk/w3sper", /@duskit\/.*/],
        },
      },
      setupFiles: ["./vite-setup.js"],
    },
  };
});
