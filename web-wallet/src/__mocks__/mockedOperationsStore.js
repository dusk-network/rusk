import { mockReadableStore } from "$lib/dusk/test-helpers";

const content = { currentOperation: "something" };

export default mockReadableStore(content);
