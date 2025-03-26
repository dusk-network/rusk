/** @overview Import the module to load all polyfills at once */

import "./asyncIterator";
import "./promiseWithResolvers";

// eslint-disable-next-line import/no-unresolved
import "web-streams-polyfill/polyfill";
