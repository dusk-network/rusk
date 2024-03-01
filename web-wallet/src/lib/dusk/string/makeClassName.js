import {
  condition,
  filterWith,
  joinWith,
  keys,
  pickIf,
  pipe,
  uniques,
} from "lamb";

const joinWithSpace = joinWith(" ");

const makeClassNameFromArray = pipe([
  filterWith(Boolean),
  uniques,
  joinWithSpace,
]);

const makeClassNameFromObject = pipe([pickIf(Boolean), keys, joinWithSpace]);

/**
 * Utility function to build a CSS class name string.<br/>
 * The function accepts either an Object or an Array.<br/>
 * If you pass an object, then it should have the class names as keys. The resulting class names will be
 * the keys holding "truthy" values.<br/>
 * "Falsy" values are <code>""</code>, <code>0</code>, <code>-0</code>, <code>false</code>,
 * <code>null</code>, <code>NaN</code> and <code>undefined</code>.
 * @example
 * const obj = { foo: true, bar: false, baz: true };
 * const arr = ["foo", undefined, "", "foo", "baz"];
 *
 * makeClassName(obj) // => "foo baz"
 * makeClassName(arr) // => "foo baz"
 * makeClassName({}) // => ""
 * makeClassName([]) // => ""
 */
const makeClassName = condition(
  Array.isArray,
  makeClassNameFromArray,
  makeClassNameFromObject
);

export default makeClassName;
