import { defineConfig, globalIgnores } from "@eslint/config-helpers";
import globals from "globals";
import jsEsLintConfig from "@dusk-network/eslint-config";
import svelteEsLintConfig from "@dusk-network/eslint-config/svelte";
import vitestEsLintConfig from "@dusk-network/eslint-config/vitest";

import svelteConfig from "./svelte.config.js";

/** @type {import("eslint").Linter.Config[]} */
export default defineConfig([
  {
    files: ["**/*.{js,mjs,cjs,svelte}"],
    languageOptions: {
      ecmaVersion: "latest",
      globals: {
        ...globals.browser,
        ...globals.node,
        CONFIG: "readonly",
      },
      sourceType: "module",
    },
    settings: {
      "import/resolver": {
        node: {},
        typescript: {
          alwaysTryTypes: true,
          project: "./jsconfig.json",
        },
      },
    },
  },
  {
    extends: [jsEsLintConfig],
    files: ["src/**/*.{js,mjs,cjs}"],
  },
  {
    extends: [svelteEsLintConfig],
    files: ["**/*.svelte"],
    languageOptions: {
      parserOptions: {
        svelteConfig,
      },
    },
  },
  {
    extends: [vitestEsLintConfig],
    files: ["*.js", "**/*.spec.js", "src/lib/dusk/mocks/**/*.js"],
  },
  globalIgnores([".svelte-kit/", "build/", "coverage/"]),
]);
