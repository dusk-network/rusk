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
    rules: {
      /**
       * This rule was added in `eslint-plugin-svelte` v3.12.0.
       * We disable it temporarily for links, as we have our own
       * path resolution in place.
       *
       * @see https://sveltejs.github.io/eslint-plugin-svelte/rules/no-navigation-without-resolve/
       */
      "svelte/no-navigation-without-resolve": [
        "error",
        {
          ignoreGoto: false,
          ignoreLinks: true,
          ignorePushState: false,
          ignoreReplaceState: false,
        },
      ],
    },
  },
  {
    extends: [vitestEsLintConfig],
    files: ["*.js", "**/*.spec.js", "src/lib/dusk/mocks/**/*.js"],
  },
  globalIgnores([".svelte-kit/", "build/", "coverage/"]),
]);
