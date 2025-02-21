import { describe, expect, it } from "vitest";

import { failureToRejection } from "..";

describe("failureToRejection", () => {
  it('should return a rejected Promise if the given Response status is not "ok"', async () => {
    const response = new Response("", { status: 404 });
    const result = failureToRejection(response);

    await expect(result).rejects.toBeInstanceOf(Error);
    await expect(result).rejects.toMatchObject({
      cause: response,
      message: expect.stringContaining(response.statusText),
    });
  });

  it('should return a Promise that resolves to the given Response, if its status is "ok"', async () => {
    const response = new Response("", { status: 200 });
    const result = failureToRejection(response);

    await expect(result).resolves.toBe(response);
  });
});
