// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { CallableProxy } from "./callable.js";
import { ListenerProxy } from "./listener.js";
import { RuesEvent } from "./event.js";
import { RuesScope } from "./scope.js";

const KEEP_ALIVE_INTERVAL = 30000;
const _rues = Symbol("rues");

const protocol = { "https:": "wss:", "http:": "ws:" };

const once = (target, topic) =>
  new Promise((resolve) =>
    target.addEventListener(topic, resolve, { once: true })
  );

class RuesTarget {
  scope;
  id;
  options;

  constructor(scope, options = {}) {
    this.scope = scope;
    this.options = options;
  }

  withId(id) {
    const target = new RuesTarget(this.scope, this.options);
    target.id = id;
    target[_rues] = this[_rues];

    return Object.freeze(target);
  }

  get rues() {
    return this[_rues];
  }

  get on() {
    return new ListenerProxy(this);
  }

  get once() {
    return new ListenerProxy(this, { once: true });
  }

  get call() {
    return new CallableProxy(this);
  }

  toString() {
    return this.scope + (this.id ? `:${this.id}` : "");
  }

  toURL() {
    return new URL(`on/${this}/`, this[_rues].url);
  }
}

export class Rues extends EventTarget {
  #url;
  #socket;
  #scopes;
  #session;
  #version = "1.0.0-rc.0";
  #keepAliveId;

  constructor(url, options = {}) {
    super();

    this.#scopes = new Map();

    if (typeof url === "string") {
      this.#url = new URL(url);
    } else if (!(url instanceof URL)) {
      throw new TypeError(`${url} is not a valid URL.`);
    } else {
      this.#url = url;
    }

    if (!["http:", "https:"].includes(this.#url.protocol)) {
      throw new TypeError(`${this.#url} is not a http(s) URL.`);
    }

    if (options.version) {
      this.#version = options.version;
    }

    this.#session = Promise.withResolvers();
  }

  #startKeepAlive() {
    this.#keepAliveId = setTimeout(() => {
      // the socket could be closing
      if (this.#socket.readyState === WebSocket.OPEN) {
        this.#socket.send("");
        this.#startKeepAlive();
      }
    }, KEEP_ALIVE_INTERVAL);
  }

  #stopKeepAlive() {
    clearTimeout(this.#keepAliveId);
  }

  get url() {
    const { protocol, hostname, port } = this.#url;

    return new URL(`${protocol}//${hostname}` + (port ? `:${port}` : ""));
  }

  get version() {
    return this.#version;
  }

  get sessionId() {
    return this.#session.promise;
  }

  static connect(url, options = {}) {
    return new Rues(url).connect(options);
  }

  async connect(options = {}) {
    if (
      [WebSocket.CONNECTING, WebSocket.OPEN].includes(this.#socket?.readyState)
    ) {
      if (options && Object.keys(options).length !== 0) {
        console.warn(
          "Options not applied: Rues is already either connected or connecting"
        );
      }

      return this.#session.promise;
    }

    const { signal } = options;

    if (signal?.aborted) {
      this.#session.reject(signal.reason);
      return this.#session.promise;
    }

    const url = this.url;

    url.protocol = protocol[url.protocol];
    url.pathname = "/on";

    const socket = new WebSocket(url);

    socket.binaryType = "arraybuffer";
    this.#socket = socket;

    socket.onerror = (errorEvent) => {
      const cause =
        errorEvent.error ?? (signal?.aborted ? signal.reason : errorEvent);

      /**
       * This handles the case where the WebSocket can't
       * establish a connection just after being created.
       */
      if (socket.readyState !== WebSocket.OPEN) {
        this.#session.reject(cause);
      } else {
        this.dispatchEvent(
          new ErrorEvent("error", {
            error: cause,
            message: cause instanceof Error ? cause.message : String(cause),
          })
        );
      }
    };

    once(socket, "close").finally(() => {
      this.#stopKeepAlive();
      this.#session = Promise.withResolvers();
      this.dispatchEvent(new CustomEvent("disconnect"));
    });

    if (signal) {
      signal.addEventListener("abort", () => {
        this.disconnect();
      });
    }

    once(socket, "open")
      .then(async () => {
        const event = await once(socket, "message");

        socket.addEventListener("message", this, { signal });
        this.#session.resolve(event.data);
        this.dispatchEvent(new CustomEvent("connect"));
        this.#startKeepAlive();
      })
      .catch((err) => {
        this.#session.reject(err);
      });

    return this.#session.promise;
  }

  async disconnect() {
    if (!this.connected) {
      return;
    }

    this.#socket.close();
    await once(this.#socket, "close");
  }

  get connected() {
    return this.#socket?.readyState === WebSocket.OPEN;
  }

  scope(source, options = {}) {
    let name;

    if (typeof source === "string") {
      name = source;
    } else if (source instanceof RuesScope) {
      ({ name } = source);
      this.#scopes.set(name, source);
    }

    const target = new RuesTarget(name, options);
    target[_rues] = this;

    return Object.freeze(target);
  }

  handleEvent(event) {
    if (event instanceof MessageEvent) {
      this.#stopKeepAlive();
      this.#startKeepAlive();

      let ruesEvent = RuesEvent.from(event);

      const scope = this.#scopes.get(ruesEvent.origin.scope);

      if (scope) {
        ruesEvent = scope.eventFrom(ruesEvent);
      }

      let ruesComponentEvent = RuesEvent.from(ruesEvent, { as: "component" });

      this.dispatchEvent(ruesEvent);
      this.dispatchEvent(ruesComponentEvent);
    }
  }
}
