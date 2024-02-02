module.exports = {
	"env": {
		"browser": true,
		"es2022": true,
		"node": true
	},
	"extends": [
		"@dusk-network/eslint-config/js",
		"@dusk-network/eslint-config/svelte"
	],
	"globals": {
		 "CONFIG": false
	},
	"overrides": [{
		"files": ["*.svelte"],
		"rules": {
			"no-undef-init": 0,
			"svelte/require-optimized-style-attribute": 0
		}
	}, {
		"files": ["*.spec.js", "*.test.js"],
		"rules": {
			"brace-style": [
				"error",
				"1tbs",
				{ "allowSingleLine": true }
			],
			"jsdoc/require-jsdoc": 0,
			"max-len": [
				"error", {
					"code": 110,
					"comments": 110,
					"ignorePattern": "^\\s*it\\(",
					"ignoreUrls": true
				}
			],
			"max-nested-callbacks": [
				"error",
				5
			],
			"max-statements": 0,
			"max-statements-per-line": [
				"error", {
					"max": 2
				}
			],
			"object-curly-newline": 0,
			"quote-props": [
				"error",
				"consistent"
			],
			"sort-keys": 0
		}
	}],
	"root": true,
	"settings": {
		"import/resolver": {
			"eslint-import-resolver-custom-alias": {
				"alias": {
					"$app": "node_modules/@sveltejs/kit/src/runtime/app",
					"$config": "./src/config",
					"$lib": "./src/lib",
					"@sveltejs/kit": "node_modules/@sveltejs/kit/src/exports/index.js",
					"@testing-library/svelte": "node_modules/@testing-library/svelte/src/index.js",
					"svelte/motion": "node_modules/svelte/src/runtime/motion/index.js",
					"svelte/store": "node_modules/svelte/src/runtime/store/index.js",
					"svelte/transition": "node_modules/svelte/src/runtime/transition/index.js"
				},
				"extensions": [".cjs", ".js", ".json", ".svelte"]
			}
		}
	}
};
