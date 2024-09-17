// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/**
 * @typedef {Object} MemoryAccessor
 *
 * @property {WebAssembly.Memory} memory - The WebAssembly memory object
 * @property {function} memcpy - The memory copy function
 * @property {Object} globals - The WebAssembly's globals
 */

/**
 * This is a callback function description.
 *
 * @callback TaskCallback
 * @param {WebAssembly.Exports} exports - The WebAssembly exports object
 * @param {MemoryAccessor} accessor- The memory accessor object
 *
 * @returns {void} - Description of the return value.
 */

/**
 * @typedef {Object} ModuleOptions
 * @property {URL|string|Uint8Array} source - The source of the WebAssembly
 */

/**
 * @typedef {Object} SandboxOptions
 * @property {AbortSignal} signal - An AbortSignal object instance
 * @property {URL|string} [importsUrl] - The URL of the imports
 */

/**
 * @typedef {Object.<string, *>} Options
 */

import { MemoryProxy, NullTarget } from "./proxies.js";
import { Sandbox } from "./sandbox/mod.js";

/**
 * Ensures at runtime the imports value is valid.
 * @private
 *
 * @param {URL|string} [value] - the imports value
 *
 * @returns {string|undefined} - the validated imports value
 */
const ensureImportsValue = (value) => {
  if (value instanceof URL) {
    return value.href;
  } else if (typeof value === "string") {
    return value;
  } else if (typeof value === "undefined" || null) {
    return undefined;
  } else {
    throw new TypeError("imports can be only a URL, a string or undefined");
  }
};

/**
 * Class representing a WebAssembly module running in isolation in a
 * {@link Sandbox}.
 *
 * All operations are asynchronous and return a Promise.
 */
export class Module {
  #module;
  #importsUrl;

  /**
   * Creates a Module instance.
   *
   * @param {ModuleOptions|ModuleOptions#source} options
   */
  constructor(options) {
    const source = options?.source ?? options;
    if (source instanceof URL) {
      this.#module = WebAssembly.compileStreaming(fetch(source.href));
    } else if (typeof source === "string") {
      this.#module = WebAssembly.compileStreaming(fetch(source));
    } else if (
      source instanceof Uint8Array ||
      Object.getPrototypeOf(source) instanceof Uint8Array
    ) {
      this.#module = WebAssembly.compile(source);
    } else {
      throw ReferenceError("`source` should be either a URL or a buffer");
    }
  }

  /**
   * The default imports URL (if any)
   *
   * @type {URL}
   */
  get defaultImports() {
    return this.#importsUrl;
  }

  set defaultImports(value) {
    this.#importsUrl = ensureImportsValue(value);
  }

  /**
   * Creates a new {Sandbox} instance for the module.
   *
   * @param {SandboxOptions} [options] - The options for the sandbox
   *
   * @returns {Promise<Sandbox>} - The sandbox instance
   */
  #createSandbox(options = {}) {
    return new Promise(async (resolve, reject) => {
      const { signal, imports } = options;

      if (signal?.aborted) {
        reject(signal.reason);
        return;
      }

      const module = await this.#module;
      const importsUrl = ensureImportsValue(imports) ?? this.#importsUrl;

      const sandbox = new Sandbox({ module, importsUrl, signal });

      resolve(sandbox);
    });
  }

  /**
   * Creates a task to run asynchronously in a newly created {Sandbox}.
   *
   * @param {TaskCallback} fn - The function to run in the sandbox
   * @returns {Promise<function>} - An asynchronous function can be run with {SandboxOptions}
   */
  task(fn) {
    return async (options = {}) => {
      const sandbox = await this.#createSandbox(options);
      const memory = await sandbox.memory;
      const exports = sandbox.exports;
      const memcpy = sandbox.memcpy;
      const globals = await sandbox.globals;

      const result = fn(exports, {
        memory: memory ?? MemoryProxy,
        memcpy,
        globals,
      });

      return Promise.resolve(result).finally(sandbox.terminate);
    };
  }

  /**
   * Executes a single method from the WebAssembly module.
   * Each call creates a new {Sandbox} instance, thus it is not suitable for
   * running multiple methods. See {Module#task} for that.
   *
   * @param {SandboxOptions} [options] - The options for the sandbox
   *
   * @returns {WebAssembly.Exports} -The WebAssembly exports object
   */
  api = (options = {}) =>
    new Proxy(NullTarget, {
      get:
        (_, prop) =>
        (...args) =>
          this.task((exports) => exports[prop](...args))(options),
    });
}
