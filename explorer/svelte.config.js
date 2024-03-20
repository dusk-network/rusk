import adapter from "@sveltejs/adapter-static";
import autoprefixer from "autoprefixer";
import postCSSNested from "postcss-nested";
import preprocess from "svelte-preprocess";
import { loadEnv } from "vite";

const env = loadEnv("", process.cwd());
const fallBackBase = env.VITE_BASE_PATH ? `${env.VITE_BASE_PATH}/` : "";

/** @type {import("@sveltejs/kit").Config} */
const config = {
  kit: {
    adapter: adapter({ fallback: `${fallBackBase}index.html` }),

    paths: {
      base: /** @type {"" | `/${string}` | undefined} */ (env.VITE_BASE_PATH),
    },
  },
  preprocess: [
    preprocess({
      postcss: {
        plugins: [autoprefixer, postCSSNested],
      },
    }),
  ],
};

export default config;
