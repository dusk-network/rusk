// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { loadWasmDataDriver } from "./loader.js";

/**
 * Data-driver registry for W3sper.
 * - register(key, locator): locator may be URL, Uint8Array, ArrayBuffer,
 *   or async () => bytes.
 * - get(key): resolves/loads, auto-init() once, caches instance.
 * Keeps driver lifecycle out of app/tests, one source of truth.
 */

/** Registry keyed by contract id (hex) or alias, instances are cached. */
export class DataDriverRegistry {
  #fetch;
  #locators = new Map();
  #instances = new Map();

  constructor(fetchImpl = fetch) {
    this.#fetch = fetchImpl;
  }

  /** Register a locator: URL | bytes | ArrayBuffer | async () => bytes */
  register(key, locator) {
    this.#locators.set(String(key), locator);
    return this;
  }

  has(key) {
    key = String(key);
    return this.#locators.has(key) || this.#instances.has(key);
  }

  /** Resolve a driver instance, if `value` is given we register it first. */
  async get(key, value) {
    key = String(key);
    if (value !== undefined) this.register(key, value);
    if (this.#instances.has(key)) return this.#instances.get(key);

    const loc = this.#locators.get(key);
    if (!loc) throw new Error(`No data‑driver registered for ${key}`);

    let bytes;
    switch (true) {
      case typeof loc === "string" || loc instanceof URL:
        {
          const res = await this.#fetch(String(loc));
          bytes = new Uint8Array(await res.arrayBuffer());
        }
        break;
      case loc instanceof Uint8Array:
        bytes = loc;
        break;
      case loc instanceof ArrayBuffer:
        bytes = new Uint8Array(loc);
        break;
      case typeof loc === "function":
        {
          const out = await loc();
          bytes = out instanceof Uint8Array ? out : new Uint8Array(out);
        }
        break;
      default:
        throw new TypeError("Unsupported driver locator type");
    }

    const driver = await loadWasmDataDriver(bytes);
    if (!driver.__w3sper_inited && typeof driver.init === "function") {
      driver.init(); // Registers the static driver instance
      Object.defineProperty(driver, "__w3sper_inited", { value: true });
    }
    this.#instances.set(key, driver);
    return driver;
  }
}
