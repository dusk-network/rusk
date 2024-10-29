import { build, stop } from "https://deno.land/x/esbuild/mod.js";

const outputDir = "./dist";

const dirs = [
  { source: "./src/mod.js", dest: `${outputDir}/w3sper.js` },
  { source: "./src/network/mod.js", dest: `${outputDir}/network.js` },
  {
    source: "./src/protocol-driver/mod.js",
    dest: `${outputDir}/protocol-driver.js`,
  },
];

try {
  if (Deno.statSync(outputDir).isDirectory) {
    Deno.removeSync(outputDir, { recursive: true });
  }
  Deno.mkdirSync(outputDir, { recursive: true });
} catch (err) {
  console.error("Error setting up the output directory:", err);
}

for (const dir of dirs) {
  try {
    const result = await build({
      entryPoints: [dir.source],
      outfile: dir.dest,
      bundle: false,
      minify: false,
      format: "esm",
      sourcemap: true,
      platform: "node",
    });

    console.log(`${dir.source} has been built:`, result);
  } catch (err) {
    console.error(`Build failed for ${dir.source}:`, err);
  }
}

stop();

const packageJsonContent = {
  name: "@dusk-network/w3sper.js",
  version: "0.0.1",
  main: "w3sper.js",
  type: "module",
  exports: {
    ".": "./w3sper.js",
    "./network": "./network.js",
    "./protocol-driver": "./protocol-driver.js",
  },
};

const packageJsonPath = `${outputDir}/package.json`;

try {
  Deno.writeTextFileSync(
    packageJsonPath,
    JSON.stringify(packageJsonContent, null, 2)
  );
  console.log("package.json has been generated:", packageJsonContent);
} catch (err) {
  console.error("Failed to write package.json:", err);
}
