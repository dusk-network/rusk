import { defineConfig, globalIgnores } from "eslint/config";
import globals from "globals";

import jsEsLintConfig from "@dusk-network/eslint-config/js/index.js";
import svelteEsLintConfig from "@dusk-network/eslint-config/svelte/index.js";
import vitestEsLintConfig from "@dusk-network/eslint-config/vitest/index.js";
import svelteConfig from "./svelte.config.js";

export default defineConfig([
  {
    extends: [jsEsLintConfig],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.es2016,
        ...globals.node,
      },
    },
    settings: {
      "import/ignore": ["node_modules"],
      "import/resolver": {
        "eslint-import-resolver-custom-alias": {
          alias: {
            $app: "node_modules/@sveltejs/kit/src/runtime/app",
            $config: "./src/config",
            $lib: "./src/lib",
            "@sveltejs/kit": "node_modules/@sveltejs/kit/src/exports/index.js",
            "@testing-library/svelte":
              "node_modules/@testing-library/svelte/src/index.js",
            "svelte/motion": "node_modules/svelte/src/runtime/motion/index.js",
            "svelte/store": "node_modules/svelte/src/runtime/store/index.js",
            "svelte/transition":
              "node_modules/svelte/src/runtime/transition/index.js",
          },
          extensions: [".cjs", ".js", ".json", ".svelte"],
        },
      },
    },
  },
  {
    extends: [svelteEsLintConfig],
    languageOptions: {
      parserOptions: {
        svelteConfig,
      },
    },
  },
  {
    extends: [vitestEsLintConfig],
  },
  globalIgnores([
    ".DS_Store",
    "build/",
    ".svelte-kit/",
    "package/",
    ".env",
    ".env.*",
    "!.env.example",
  ]),
]);
