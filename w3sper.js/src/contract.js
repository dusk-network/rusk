// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as base16 from "./encoders/b16.js";

// BigInt-safe JSON (u64/u128 as strings)
function jsonWithBigInts(value) {
  return JSON.stringify(
    value,
    (_, v) => (typeof v === "bigint" ? v.toString() : v),
  );
}

/**
 * Minimal contract facade for data-driver based contracts.
 *
 * - call.<fn>(args?)            -> JSON, read-only calls
 * - tx.<fn>(args?)              -> Transfer builder (payload prefilled)
 * - events.<event>.once()/on()  -> decoded events in JSON
 * - schema()/version()          -> driver metadata
 *
 * This wrapper class around data-drivers hides JSON <-> RKYV and content-type
 * details behind simple methods.
 */
export class Contract {
  #idHex;
  #idBytes;
  #driverPromise;
  #network;
  #bookentry;

  constructor({ contractId, driver, network, bookentry }) {
    switch (true) {
      case typeof contractId === "string":
        this.#idHex = contractId.toLowerCase();
        this.#idBytes = base16.decode(contractId);
        break;
      case contractId instanceof Uint8Array:
        this.#idBytes = contractId;
        this.#idHex = base16.encode(contractId);
        break;
      default:
        throw new TypeError("contractId must be hex string or Uint8Array");
    }

    this.#driverPromise = Promise.resolve(driver);
    this.#network = network ?? null;
    this.#bookentry = bookentry ?? null;
  }

  get id() {
    return this.#idHex;
  }

  // Driver metadata
  async schema() {
    const d = await this.#driverPromise;
    return d.getSchema?.();
  }
  async version() {
    const d = await this.#driverPromise;
    return d.getVersion?.();
  }

  /**
  * Encode a function's input using the contract's data-driver (JSON -> RKYV).
  * Returns Uint8Array of RKYV bytes.
  */
  async encode(fnName, jsonValue) {
    const driver = await this.#driverPromise;
    const json = (jsonValue === undefined || jsonValue === null)
      ? "null"
      : jsonWithBigInts(jsonValue);
    return driver.encodeInputFn(String(fnName), json);
  }

  /**
  * Decode an input payload using the contract's data-driver (RKYV -> JSON).
  * Returns a JSON value (JS object/array/primitive).
  */
  async decode(fnName, rkyvBytes) {
    const loader = await this.#driverPromise;
    return await loader.decodeInputFn(fnName, rkyvBytes);
  }

  #payloadToBytes(evt) {
    const p = evt?.payload;
    if (!p) return null;
    if (p instanceof Uint8Array) return p;
    if (p instanceof ArrayBuffer) return new Uint8Array(p);
    if (typeof p === "string") {
      const isHex = /^[0-9a-fA-F]+$/.test(p) && p.length % 2 === 0;
      if (isHex) return base16.decode(p);
      try {
        const bin = atob(p);
        const buf = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
        return buf;
      } catch { /* ignore */ }
    }
    return null; // object => already JSON (no bytes to decode)
  }

  get call() {
    return new Proxy({}, {
      get: (_t, fnName) => async (args = undefined) => {
        if (!this.#network) {
          throw new Error("call requires a Network provider");
        }
        const rkvy = await this.encode(fnName, args);
        const resp = await this.#network.contracts
          .withId(this.#idHex).call[String(fnName)](rkvy);
        const bytes = new Uint8Array(await resp.arrayBuffer());
        const driver = await this.#driverPromise;
        return driver.decodeOutputFn(String(fnName), bytes);
      },
    });
  }

  get tx() {
    return new Proxy({}, {
      get: (_t, fnName) => async (args = undefined) => {
        if (!this.#bookentry) {
          throw new Error("tx requires a Bookkeeper entry (profile)");
        }
        const rkvy = await this.encode(fnName, args);
        const payload = Object.freeze({
          fnName: String(fnName),
          fnArgs: rkvy,
          contractId: Array.from(this.#idBytes),
        });
        return this.#bookentry.transfer(0n).payload(payload);
      },
    });
  }

  get events() {
    const requireNetwork = () => {
      if (!this.#network) throw new Error("events requires a Network provider");
      return this.#network.contracts.withId(this.#idHex);
    };

    const decode = async (name, evt) => {
      const bytes = this.#payloadToBytes(evt);
      if (!bytes) return evt.payload;
      const driver = await this.#driverPromise;
      return driver.decodeEvent(name, bytes);
    };

    const apiFor = (name) => ({
      once: async () => {
        const contracts = requireNetwork();
        const evt = await contracts.once[name]();
        return decode(name, evt);
      },
      on: (handler) => {
        const contracts = requireNetwork();
        const stop = contracts.on[name](async (evt) => {
          try {
            handler(await decode(name, evt));
          } catch (e) {
            handler(undefined, e);
          }
        });
        return stop;
      },
    });

    return new Proxy({}, {
      get: (_t, prop) => apiFor(String(prop)),
    });
  }
}
