const moduleUrl = new URL(import.meta.url);
const paths = moduleUrl.pathname.split("/");
const testsIndex = paths.lastIndexOf("tests");

if (testsIndex === -1) {
  console.error("tests folder missing in path: ", moduleUrl.pathname);
  Deno.exit(1);
}

const baseUrl = new URL(paths.slice(0, testsIndex + 1).join("/"), moduleUrl);
const srcUrl = new URL(paths.slice(0, testsIndex).join("/"), moduleUrl);

// Start listening on port 8000 of localhost.
startServer();
console.log("File server running on http://localhost:8000/");

function startServer() {
  Deno.serve(async (req) => {
    // Use the request pathname as filepath
    const url = new URL(req.url);
    let filepath = decodeURIComponent(url.pathname);

    if (filepath === "/" || filepath === "/index.html") {
      filepath = baseUrl.pathname + "/index.html";
    } else if (filepath.startsWith("/src/")) {
      filepath = srcUrl.pathname + filepath;
    } else {
      filepath = baseUrl.pathname + filepath;
    }

    // Try opening the file
    let file;
    try {
      file = await Deno.open(filepath, { read: true });
    } catch {
      // If the file cannot be opened, return a "404 Not Found" response
      return new Response("404 Not Found", { status: 404 });
    }

    // Build a readable stream so the file doesn't have to be fully loaded into
    // memory while we send it
    const readableStream = file.readable;

    const options = {
      headers: new Headers({
        "Cross-Origin-Opener-Policy": "same-origin",
        "Cross-Origin-Embedder-Policy": "require-corp",
      }),
    };

    if (filepath.endsWith(".wasm")) {
      options.headers.set("Content-Type", "application/wasm");
    } else if (filepath.endsWith(".js")) {
      options.headers.set("Content-Type", "application/javascript");
    }

    // Build and send the response
    return new Response(readableStream, options);
  });
}
