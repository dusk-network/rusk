import { defineConfig } from "@eslint/config-helpers";
import globals from "globals";
import jsEsLintConfig from "@dusk-network/eslint-config";

/** @type {import("eslint").Linter.Config[]} */
export default defineConfig([
  {
    files: ["**/*.{js,mjs,cjs}"],
    languageOptions: {
      ecmaVersion: "latest",
      globals: {
        ...globals.node,
      },
      sourceType: "module",
    },
    settings: {
      "import/resolver": {
        node: {},
      },
    },
  },
  {
    extends: [jsEsLintConfig],
    files: ["src/**/*.{js,mjs,cjs}"],
  },
]);
