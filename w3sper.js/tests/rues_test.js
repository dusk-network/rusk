import {
  test,
  assert,
} from "http://rawcdn.githack.com/mio-mini/test-harness/0.1.0/mod.js";

import { Rues } from "../src/rues.js";

test("rues", async () => {
  // Usage
  const rues = await Rues.connect("http://localhost:8080");
  console.log(rues);
  console.log(
    await rues.invoke.node.info().then((response) => response.json()),
  );

  await new Promise((r) => setTimeout(r, 100_000));
  await rues.disconnect();

  assert.ok("hello");
});
