// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export default function () {
  const addListener = addEventListener.bind(self, "message");
  const removeListener = removeEventListener.bind(self, "message");

  const Internals = {
    instance: null,
    imports: null,
    memoryPort: null,
  };

  const getGlobals = (module) =>
    WebAssembly.Module.exports(module)
      .filter(({ kind, name }) => kind === "global" && !name.startsWith("__"))
      .reduce(
        (acc, item) => (
          (acc[item.name] = Internals.instance.exports[item.name].value), acc
        ),
        {}
      );

  async function init({ module, importsUrl }, [port]) {
    if (typeof importsUrl === "string") {
      const importsModule = await import(importsUrl);
      Internals.imports = importsModule.default;

      Internals.instance = new WebAssembly.Instance(module, Internals.imports);

      if (typeof importsModule.oninit === "function") {
        await importsModule.oninit(Internals.instance);
      }
    } else {
      Internals.instance = new WebAssembly.Instance(module);
    }

    Internals.memoryPort = port;
    Internals.memoryPort.onmessage = handleMemoryRequest;
  }

  function handleMemoryRequest({ data: { get, set } }) {
    const memory =
      Internals.imports?.env?.memory ?? Internals.instance?.exports?.memory;

    if (!(memory instanceof WebAssembly.Memory)) {
      throw new ReferenceError("WebAssembly.Memory is not defined");
    } else if (set) {
      const { dest, source, count } = set;
      const length = count ?? source.byteLength ?? source.length;

      new Uint8Array(memory.buffer, dest, length).set(source);
      Internals.memoryPort.postMessage(source, [source.buffer]);
    } else if (get) {
      const { source, count } = get;
      const length = count ?? source.byteLength ?? source.length;
      Internals.memoryPort.postMessage(
        new Uint8Array(memory.buffer.slice(source, source + length))
      );
    } else {
      throw new TypeError("Invalid memory request");
    }
  }

  function handleRequest({ data }) {
    const { member, args } = data;
    const method = Internals.instance.exports[member];

    if (typeof method === "function") {
      const result = method(...args);

      postMessage(result);
    } else {
      postMessage(new TypeError(`${member} is not a function`));
    }
  }

  // Module entry point
  addListener(async function main({ data, ports }) {
    await init(data, ports);
    removeListener(main);
    addListener(handleRequest);

    const initialized = {
      memory: null,
      globals: getGlobals(data.module),
    };

    // `crossOriginIsolated` might be `undefined`; in that case, the behavior is
    // the same as having the value `true`.
    if (self.crossOriginIsolated !== false) {
      initialized.memory = Internals.imports?.env?.memory ?? null;
    }

    postMessage(initialized);
  });
}
