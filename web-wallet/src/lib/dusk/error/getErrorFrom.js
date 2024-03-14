import {
  adapter,
  always,
  casus,
  identity,
  isNil,
  isType,
  keySatisfies,
} from "lamb";

/** @type {(value: any) => value is Error} */
const isError = isType("Error");

/** @type {(value: any) => value is string} */
const isString = isType("String");

/** @type {(value: any) => Error} */
const getErrorFrom = adapter([
  casus(isError, identity),
  casus(isNil, always(new Error("Unknown error"))),
  casus(isString, (msg) => new Error(msg)),
  casus(keySatisfies(isString, "message"), ({ message }) => new Error(message)),
  (v) => new Error(JSON.stringify(v)),
]);

export default getErrorFrom;
