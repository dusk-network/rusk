import { build, stop } from "https://deno.land/x/esbuild/mod.js";

const entryPoint = "./src/mod.js";

const outputDir = "./dist";
const outputFile = `${outputDir}/w3sper.js`;

if (Deno.statSync(outputDir).isDirectory) {
  Deno.removeSync(outputDir, { recursive: true });
}

Deno.mkdirSync(outputDir, { recursive: true });

// Build the SDK using esbuild
const result = await build({
  entryPoints: [entryPoint],
  outfile: outputFile,
  bundle: true,
  minify: false,
  format: "esm",
  sourcemap: true,
});

console.log("w3sper SDK has been built:", result);

stop();

const packageJsonContent = {
  name: "w3sper",
  version: "0.0.1",
  main: "w3sper.js",
  type: "module",
  exports: {
    ".": "./w3sper.js",
  },
};

const packageJsonPath = "./dist/package.json";

Deno.writeTextFileSync(
  packageJsonPath,
  JSON.stringify(packageJsonContent, null, 2)
);

console.log("package.json has been generated:", packageJsonContent);
