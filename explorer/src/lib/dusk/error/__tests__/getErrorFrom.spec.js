import { describe, expect, it } from "vitest";

import { getErrorFrom } from "..";

describe("getErrorFrom", () => {
  it("should return the received argument if it's an instance of `Error`", () => {
    [
      class CustomError extends Error {},
      Error,
      EvalError,
      RangeError,
      ReferenceError,
      SyntaxError,
      TypeError,
      URIError,
    ].forEach((Ctor) => {
      const error = new Ctor("some error message");

      expect(getErrorFrom(error)).toBe(error);
    });

    const aggregateError = new AggregateError([new Error("foo message")]);

    expect(getErrorFrom(aggregateError)).toBe(aggregateError);
  });

  it('should return an error having "Unknown error" as its message if the received argument is `null` or `undefined`', () => {
    const expectedError = new Error("Unknown error");

    expect(getErrorFrom(null)).toStrictEqual(expectedError);
    expect(getErrorFrom(undefined)).toStrictEqual(expectedError);
  });

  it("should return an Error if the received argument is a string and use it as the error message", () => {
    const msg = "Some error message";
    const result = getErrorFrom(msg);

    expect(result).toStrictEqual(new Error(msg));
  });

  it("should return an Error if the received argument is an object with a message string property, and use it as the error message", () => {
    const err = { message: "Some error message" };
    const result = getErrorFrom(err);

    expect(result).toStrictEqual(new Error(err.message));
  });

  it("should return an Error otherwise using a JSON representation of the received argument as the error message", () => {
    [
      123,
      new Date(),
      /a/g,
      { foo: "bar" },
      [1, 2, 3],
      { message: 345 },
    ].forEach((arg) => {
      const result = getErrorFrom(arg);

      expect(result).toStrictEqual(new Error(JSON.stringify(arg)));
    });
  });
});
