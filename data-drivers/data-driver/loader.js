// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

export async function loadDriverWasm(bytes) {
  const wasmModule = await WebAssembly.instantiate(bytes, { env: {} });
  const exports = wasmModule.instance.exports;
  const memory = exports.memory;

  const textEncoder = new TextEncoder();
  const textDecoder = new TextDecoder();

  // -- Helpers --
  function allocAndWriteString(str) {
    const strBytes = textEncoder.encode(str);
    const ptr = exports.alloc(strBytes.length);
    new Uint8Array(memory.buffer, ptr, strBytes.length).set(strBytes);
    return [ptr, strBytes.length];
  }

  function allocAndWriteBytes(byteArray) {
    const ptr = exports.alloc(byteArray.length);
    new Uint8Array(memory.buffer, ptr, byteArray.length).set(byteArray);
    return [ptr, byteArray.length];
  }

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

    /** Returns the contract's JSON schema */
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
