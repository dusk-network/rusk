import adapter from "@sveltejs/adapter-static";
import autoprefixer from "autoprefixer";
import postCSSNested from "postcss-nested";
import preprocess from "svelte-preprocess";

/** @type {import("@sveltejs/kit").Config} */
const config = {
	kit: {
		adapter: adapter({ fallback: "index.html" })
	},
	preprocess: [
		preprocess({
			postcss: {
				plugins: [
					autoprefixer,
					postCSSNested
				]
			}
		})
	]
};

export default config;
