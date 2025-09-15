// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/**
 * Data‑driver WASM loader (JS runtime bindings)
 *
 * This JS module wires a compiled data‑driver WASM binary to a friendly JS API.
 *
 * A “data‑driver” is a WASM module that knows how to:
 *  - encode JSON inputs for a contract function -> RKYV bytes
 *  - decode RKYV bytes from inputs/outputs/events -> JSON
 * It does not perform on‑chain calls; it only (de)serializes according to the
 * contract's ABI exposed via the Rust `ConvertibleContract` trait.
 *
 * ### Expected WASM exports
 * The WASM must export (ABI stable) functions:
 *   - `memory: WebAssembly.Memory`
 *   - `alloc(len: i32) -> i32` and `dealloc(ptr: i32, len: i32)`
 *   - `get_last_error(out_ptr: i32, out_size: i32) -> void`
 *   - `encode_input_fn(fn_ptr, fn_len, json_ptr, json_len, out_ptr, out_size) -> i32`
 *   - `decode_input_fn(fn_ptr, fn_len, rkyv_ptr, rkyv_len, out_ptr, out_size) -> i32`
 *   - `decode_output_fn(fn_ptr, fn_len, rkyv_ptr, rkyv_len, out_ptr, out_size) -> i32`
 *   - `decode_event(event_ptr, event_len, rkyv_ptr, rkyv_len, out_ptr, out_size) -> i32`
 *   - `get_schema(out_ptr, out_size) -> i32`
 *   - `get_version(out_ptr, out_size) -> i32`
 *
 * Each FFI function writes its result into `out_ptr` as:
 *   [4‑byte little‑endian length][payload bytes...]
 * and returns a status code: `0` = success, non‑zero = failure. On failure,
 * `get_last_error(...)` returns a UTF‑8 error message.
 *
 * ### Usage
 * ```js
 * import { loadDriverWasm } from "./data-driver/loader.js";
 * const bytes = await fetch("/path/to/driver.wasm").then(r => r.arrayBuffer());
 * const driver = await loadDriverWasm(new Uint8Array(bytes));
 * driver.init?.(); // optional, if exported
 * const callBytes = driver.encodeInputFn("fn_name", JSON.stringify({ amount: "100" }));
 * const schema = driver.getSchema();
 * console.log(schema, callBytes);
 * ```
 */

/**
 * Create a JS driver from a compiled WASM binary.
 * @param {Uint8Array|ArrayBuffer} bytes - The compiled data‑driver WASM.
 * @returns {Promise<{
 *   init: () => number|void,
 *   encodeInputFn: (fnName: string, json: string) => Uint8Array,
 *   decodeInputFn: (fnName: string, rkyvBytes: Uint8Array) => any,
 *   decodeOutputFn: (fnName: string, rkyvBytes: Uint8Array) => any,
 *   decodeEvent: (eventName: string, rkyvBytes: Uint8Array) => any,
 *   getSchema: () => any,
 *   getVersion: () => string
 * }>} Driver object mirroring the Rust `ConvertibleContract` methods.
 */
export async function loadDriverWasm(bytes) {
  const wasmModule = await WebAssembly.instantiate(bytes, { env: {} });
  const exports = wasmModule.instance.exports;
  const memory = exports.memory;

  const textEncoder = new TextEncoder();
  const textDecoder = new TextDecoder();

  // -- Helpers --
  /**
   * Allocate WASM memory and write a string.
   * @param {string} str
   * @returns {[ptr: number, len: number]}
   */
  function allocAndWriteString(str) {
    const strBytes = textEncoder.encode(str);
    const ptr = exports.alloc(strBytes.length);
    new Uint8Array(memory.buffer, ptr, strBytes.length).set(strBytes);
    return [ptr, strBytes.length];
  }

  /**
   * Allocate WASM memory and copy raw bytes.
   * @param {Uint8Array} byteArray
   * @returns {[ptr: number, len: number]}
   */
  function allocAndWriteBytes(byteArray) {
    const ptr = exports.alloc(byteArray.length);
    new Uint8Array(memory.buffer, ptr, byteArray.length).set(byteArray);
    return [ptr, byteArray.length];
  }

  /**
   * Read `[u32_le length][payload...]` from WASM memory at `ptr`.
   * The first 4 bytes contain the actual payload length.
   * @param {number} ptr - start of buffer we allocated for the WASM to fill
   * @param {number} bufSize - total size of that buffer (including 4‑byte header)
   * @returns {Uint8Array} a cloned view containing only the payload
   */
  function readBuffer(ptr, bufSize) {
    const view = new DataView(memory.buffer, ptr, 4);
    const actualSize = view.getUint32(0, true);

    if (actualSize > bufSize - 4) {
      exports.dealloc(ptr, bufSize);
      throw new Error(`Invalid output size: ${actualSize}`);
    }

    const data = new Uint8Array(memory.buffer, ptr + 4, actualSize);
    const result = data.slice(); // clone to detach from WASM memory
    return result;
  }

  /**
   * Allocate a temporary output buffer, run an FFI call, check return code,
   * and parse the output according to the `[len][payload]` convention.
   */
  function runWithOutputBuffer(fn) {
    const outSize = 64 * 1024; // 64 KB default buffer
    const outPtr = exports.alloc(outSize);
    try {
      const code = fn(outPtr, outSize);
      if (code !== 0) {
        const errMsg = _internalGetLastError();
        throw new Error(`FFI call failed (${code}): ${errMsg}`);
      }
      return readBuffer(outPtr, outSize);
    } finally {
      exports.dealloc(outPtr, outSize);
    }
  }

  function _internalGetLastError() {
    const outSize = 1024;
    const outPtr = exports.alloc(outSize);
    try {
      exports.get_last_error(outPtr, outSize);
      const result = readBuffer(outPtr, outSize);
      return textDecoder.decode(result);
    } finally {
      exports.dealloc(outPtr, outSize);
    }
  }

  return {
    /** Decodes RKYV event bytes into JSON */
    decodeEvent: (eventName, rkyvBytes) => {
      const [eventPtr, eventLen] = allocAndWriteString(eventName);
      const [rkyvPtr, rkyvLen] = allocAndWriteBytes(rkyvBytes);

      const result = runWithOutputBuffer((outPtr, outSize) =>
        exports.decode_event(
          eventPtr,
          eventLen,
          rkyvPtr,
          rkyvLen,
          outPtr,
          outSize
        )
      );

      exports.dealloc(eventPtr, eventLen);
      exports.dealloc(rkyvPtr, rkyvLen);

      return JSON.parse(textDecoder.decode(result));
    },

    /** Decodes RKYV input bytes into JSON */
    decodeInputFn: (fnName, rkyvBytes) => {
      const [fnPtr, fnLen] = allocAndWriteString(fnName);
      const [rkyvPtr, rkyvLen] = allocAndWriteBytes(rkyvBytes);

      const result = runWithOutputBuffer((outPtr, outSize) =>
        exports.decode_input_fn(fnPtr, fnLen, rkyvPtr, rkyvLen, outPtr, outSize)
      );

      exports.dealloc(fnPtr, fnLen);
      exports.dealloc(rkyvPtr, rkyvLen);

      return JSON.parse(textDecoder.decode(result));
    },

    /** Decodes RKYV output bytes into JSON */
    decodeOutputFn: (fnName, rkyvBytes) => {
      const [fnPtr, fnLen] = allocAndWriteString(fnName);
      const [rkyvPtr, rkyvLen] = allocAndWriteBytes(rkyvBytes);

      const result = runWithOutputBuffer((outPtr, outSize) =>
        exports.decode_output_fn(
          fnPtr,
          fnLen,
          rkyvPtr,
          rkyvLen,
          outPtr,
          outSize
        )
      );

      exports.dealloc(fnPtr, fnLen);
      exports.dealloc(rkyvPtr, rkyvLen);

      return JSON.parse(textDecoder.decode(result));
    },

    /** Encodes JSON input into RKYV bytes */
    encodeInputFn: (fnName, json) => {
      const [fnPtr, fnLen] = allocAndWriteString(fnName);
      const [jsonPtr, jsonLen] = allocAndWriteString(json);

      const result = runWithOutputBuffer((outPtr, outSize) =>
        exports.encode_input_fn(fnPtr, fnLen, jsonPtr, jsonLen, outPtr, outSize)
      );

      exports.dealloc(fnPtr, fnLen);
      exports.dealloc(jsonPtr, jsonLen);

      return result; // returns Uint8Array
    },

    /** Returns the contract's JSON schema */
    getSchema: () => {
      const result = runWithOutputBuffer((outPtr, outSize) =>
        exports.get_schema(outPtr, outSize)
      );
      return JSON.parse(textDecoder.decode(result));
    },

    /** Returns the contract's semantic version string */
    getVersion: () => {
      const result = runWithOutputBuffer((outPtr, outSize) =>
        exports.get_version(outPtr, outSize)
      );
      return textDecoder.decode(result);
    },

    /** Initializes the contract driver */
    init: () => exports.init(),
  };
}
