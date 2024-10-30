/* eslint-disable import/no-unresolved */

import { sveltekit } from "@sveltejs/kit/vite";
import { coverageConfigDefaults } from "vitest/config";

/* eslint-enable import/no-unresolved */

import { defineConfig, loadEnv } from "vite";
import basicSsl from "@vitejs/plugin-basic-ssl";
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
        MODE_MAINTENANCE: env.VITE_MODE_MAINTENANCE,
        VITE_FEATURE_ALLOCATE: env.VITE_FEATURE_ALLOCATE,
        VITE_FEATURE_MIGRATE: env.VITE_FEATURE_MIGRATE,
        VITE_FEATURE_STAKE: env.VITE_FEATURE_STAKE,
        VITE_FEATURE_TRANSFER: env.VITE_FEATURE_TRANSFER,
        VITE_GAS_LIMIT_DEFAULT: env.VITE_GAS_LIMIT_DEFAULT,
        VITE_GAS_LIMIT_LOWER: env.VITE_GAS_LIMIT_LOWER,
        VITE_GAS_LIMIT_UPPER: env.VITE_GAS_LIMIT_UPPER,
        VITE_GAS_PRICE_DEFAULT: env.VITE_GAS_PRICE_DEFAULT,
        VITE_GAS_PRICE_LOWER: env.VITE_GAS_PRICE_LOWER,
        VITE_GAS_PRICE_UPPER: env.VITE_GAS_PRICE_UPPER,
        VITE_MINIMUM_ALLOWED_STAKE: env.VITE_MINIMUM_ALLOWED_STAKE,
        VITE_NODE_URL: env.VITE_NODE_URL,
      },
    },
    plugins:
      mode === "development" ? [basicSsl(), ...commonPlugins] : commonPlugins,
    server: {
      proxy: {
        "/on": {
          target: "ws://localhost:8080/",
          ws: true,
        },
        "/rusk": {
          rewrite: (path) => path.replace(/^\/rusk/, ""),
          target: "http://localhost:8080/",
        },
        "/static/drivers": {
          target: "http://localhost:8080/",
        },
      },
    },
    test: {
      /** @see https://github.com/vitest-dev/vitest/issues/2834 */
      alias: [{ find: /^svelte$/, replacement: "svelte/internal" }],
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
        VITE_FEATURE_MIGRATE: "true",
        VITE_FEATURE_STAKE: "true",
        VITE_FEATURE_TRANSFER: "true",
        VITE_GAS_LIMIT_DEFAULT: "20000000",
        VITE_GAS_LIMIT_LOWER: "10000000",
        VITE_GAS_LIMIT_UPPER: "1000000000",
        VITE_GAS_PRICE_DEFAULT: "1",
        VITE_GAS_PRICE_LOWER: "1",
        VITE_MINIMUM_ALLOWED_STAKE: "1234",
        VITE_NODE_URL: "",
      },
      environment: "jsdom",
      include: ["src/**/*.{test,spec}.{js,ts}"],
      setupFiles: ["./vite-setup.js"],
    },
  };
});
