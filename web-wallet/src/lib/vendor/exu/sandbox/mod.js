// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { NullTarget } from "../proxies.js";
import worker from "./worker.js";

// Create a Blob URL for the worker code.
const workerUrl = URL.createObjectURL(
  new Blob([`(${worker})()`], { type: "application/javascript" })
);

/**
 * Creates a sandbox environment for executing code in a separate context
 * using Web Workers.
 */
export class Sandbox {
  #worker;
  #globals;
  #memory;
  #memoryPort;
  #signal;

  /**
   * Constructs the sandbox and initializes its worker and memory channel.
   *
   * @param {Object} config - Configuration object for the sandbox.
   * @param {string} config.module - The module URL to load in the worker.
   * @param {string} config.importsUrl - The URL for the module's imports.
   * @param {AbortSignal} config.signal - An optional AbortSignal to cancel operations.
   */
  constructor({ module, importsUrl, signal }) {
    this.#worker = new Worker(workerUrl, { type: "module" });

    const mc = new MessageChannel();
    this.#memoryPort = mc.port1;

    const initialized = this.send(this.#worker, { module, importsUrl }, [
      mc.port2,
    ]);

    this.#signal = signal;
    this.#memory = initialized.then(({ memory }) => memory);
    this.#globals = initialized.then(({ globals }) => globals);
  }

  /**
   * Gets asynchrously the exports object from the WebAssembly module.
   *
   * @type {Promise<WebAssembly.Exports>}
   */
  get exports() {
    return new Proxy(NullTarget, {
      get:
        (_, prop) =>
        (...args) =>
          this.send(this.#worker, { member: prop, args }),
    });
  }

  get globals() {
    return this.#globals;
  }

  /**
   * Gets asynchrously the memory object for the WebAssembly module.
   *
   * @type {Promise<WebAssembly.Memory>}
   */
  get memory() {
    return this.#memory;
  }

  /**
   * Copies memory between the WebAssembly and JavaScript contexts.
   *
   * @param {number|Uint8Array|null} dest - The destination memory address or buffer.
   * @param {number|Uint8Array} source - The source memory address or buffer.
   * @param {number} [count] - The number of bytes to copy.
   *
   * @returns {Promise<void|Uint8Array>} A promise that resolves once the operation is complete.
   *
   * @throws {TypeError} Throws if the arguments are invalid.
   */
  memcpy = async (dest, source, count) => {
    if (dest === null && typeof source === "number") {
      // Copy from WASM memory to JS memory
      return await this.send(this.#memoryPort, {
        get: { source, count },
      });
    } else if (typeof dest === "number" && source instanceof Uint8Array) {
      // Copy from JS memory to WASM memory

      return await this.send(
        this.#memoryPort,
        { set: { dest, source, count } },
        [source.buffer]
      );
    } else if (typeof dest === "number" && typeof source === "number") {
      // Copy from WASM memory to WASM memory
      await this.send(this.#memoryPort, { set: { source, dest, count } });
    } else if (dest instanceof Uint8Array && source instanceof Uint8Array) {
      // Copy from JS memory to JS memory
      dest.set(source);
    } else {
      throw new TypeError("Invalid arguments.");
    }
  };

  /**
   * Sends a message to the specified receiver (worker or message port).
   *
   * @param {Worker|MessagePort} receiver - The message receiver.
   * @param {...*} args - See `postMessage` arguments.
   *
   * @returns {Promise<any>} A promise that resolves with the response from the receiver.
   */
  send = (receiver, ...args) =>
    new Promise((resolve, reject) => {
      const signal = this.#signal;

      if (signal?.aborted) {
        reject(signal.reason);
        return;
      }

      signal?.addEventListener("abort", () => {
        reject(signal.reason);
        this.terminate();
      });

      receiver.onmessage = ({ data }) => {
        data instanceof Error ? reject(data) : resolve(data);
        receiver.onmessage = null;
      };

      receiver.postMessage(...args);
    });

  terminate = () => {
    this.#memoryPort.close();
    this.#worker.terminate();
  };
}
