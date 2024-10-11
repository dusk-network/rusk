// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// /on/transactions/propagate
// /on/network/peers
// /on/node/info
// /on/blocks/gas-price
// on/prover/prove
// /on/transactions/preverify
// on/node/crs
// /on/node/provisioners*
// /on/graphql/query

const _rues = Symbol("rues");
const _once = Symbol("rues::once");

const protocol = { "https:": "wss:", "http:": "ws:" };

const once = (target, topic) =>
  new Promise((resolve) =>
    target.addEventListener(topic, resolve, { once: true }),
  );

const subscribe = async (target) => {
  const eventURL = new URL(
    `/on/${target.eventTarget}/${target.eventTopic}`,
    target[_rues].url,
  );

  const headers = new Headers();
  headers.append("rusk-version", target[_rues].ruskVersion.toString());
  headers.append("rusk-session-id", await target[_rues].sessionId);

  const response = await fetch(eventURL, { headers });

  if (!response.ok) {
    switch (response.status) {
      case 400:
        throw new Error("Unable to subscribe: Rusk-Version incompatibility");
      case 424:
        throw new Error("Unable to subscribe: Rusk-Session-Id issues");
      case 404:
        throw new Error("Unable to subscribe: Target  not found");
      default:
        throw new Error(
          `Inable to subscribe: Unknown Error ${response.status}: ${response.statusText}`,
        );
    }
  }

  return eventURL;
};

const unsubscribe = async (target) => {
  const eventURL = new URL(
    `/on/${target.eventTarget}/${target.eventTopic}`,
    target[_rues].url,
  );

  const headers = new Headers();
  headers.append("rusk-version", target[_rues].ruskVersion.toString());
  headers.append("rusk-session-id", await target[_rues].sessionId);

  const response = await fetch(eventURL, { method: "DELETE", headers });

  if (!response.ok) {
    switch (response.status) {
      case 400:
        throw new Error("Unable to unsubscribe: Rusk-Version incompatibility");
      case 424:
        throw new Error("Unable to unsubscribe: Rusk-Session-Id issues");
      case 404:
        throw new Error("Unable to unsubscribe: Target or topic not found");
      default:
        throw new Error(
          `Unable to unsubscribe: Unknown Error ${response.status}: ${response.statusText}`,
        );
    }
  }

  return eventURL;
};

const listenerTopicHandler = {
  get(target, prop) {
    target.eventTopic = prop;
    const topic = async (listener, options = {}) => {
      const { signal } = options;

      if (signal?.aborted) {
        return;
      }

      const eventURL = await subscribe(target);

      if (target[_once]) {
        target[_rues].addEventListener(
          eventURL.pathname,
          target[_once].resolve,
          {
            ...options,
            once: true,
          },
        );
        return target[_once].promise;
      } else {
        target[_rues].addEventListener(eventURL.pathname, listener, options);
      }

      if (signal) {
        signal.addEventListener("abort", async (event) => {
          await unsubscribe(target);
        });
      }
    };

    return topic;
  },
};

const listenerComponentHandler = {
  get(target, prop) {
    return (identifier) => {
      if (!identifier) {
        target.eventTarget = prop;
      } else {
        target.eventTarget = `${prop}:${identifier}`;
      }
      return new Proxy(target, listenerTopicHandler);
    };
  },
};

const dispatcherTopicHandler = {
  get(target, prop) {
    target.eventTopic = prop;
    const topic = async (body, options = {}) => {
      const { signal } = options;

      if (signal?.aborted) {
        return;
      }

      const eventURL = new URL(
        `/on/${target.eventTarget}/${target.eventTopic}`,
        target[_rues].url,
      );

      // const headers = new Headers(resource.headers);
      const headers = new Headers();
      headers.append("rusk-version", "0.8.0");
      headers.append("rusk-session-id", target[_rues].sessionId);

      const response = await fetch(eventURL, {
        method: "POST",
        body,
        headers,
        signal,
      });

      // TODO: In case of mismatching rusk versions, the node *should* return a
      // 4xx status code, however currently it always return a 500 no matter what.
      // We can't rely on the status code to determine the error, so we have to
      // check the response body.
      // This should be fixed on node side.
      if (!response.ok) {
        // We only want to check if this is a version mismatch, but since we
        // have to *consume* the body stream in order to check it, we have to
        // clone the response in case it's not a version mismatch.'

        const resp = response.clone();
        const body = await resp.text();

        if (body.startsWith("Mismatched rusk version:")) {
          throw new Error(body);
        } else {
          throw new NodeError(body);
        }
      }

      return response;

      // return "handle" in resource ? resource.handle(response) : response;
    };

    return topic;
  },
};

const dispatcherComponentHandler = {
  get(target, prop) {
    target.eventTarget = prop;

    return new Proxy(target, dispatcherTopicHandler);
  },
};

function parse(pathname) {
  const [, target, topic] = pathname.match(/^\/?on\/([^\\]+)\/(.+)/) ?? [];

  let [component, ...element] = target?.split(":") ?? [];

  return target
    ? {
        component,
        element: element.join(":"),
        topic,
      }
    : null;
}

export class NodeError extends Error {
  constructor(message) {
    const [, name, description] = message.match(
      /([A-Za-z\-]+?[ ]?Error)[: ]? ?(.*)/,
    ) ?? [, "NodeError", message];

    super(description);
    this.name = name;
  }
}

class RuesEvent extends Event {
  #headers;
  #payload;

  constructor(type) {
    super(type);
  }

  get headers() {
    return this.#headers;
  }

  get payload() {
    return this.#payload;
  }

  get origin() {
    return this.headers.get("content-location");
  }

  static from(event, options = {}) {
    if (event instanceof MessageEvent) {
      const { data } = event;
      const headersLength = new DataView(data).getUint32(0, true);
      const headersBuffer = new Uint8Array(data, 4, headersLength);
      const headers = new Headers(
        JSON.parse(new TextDecoder().decode(headersBuffer)),
      );
      const body = new Uint8Array(data, 4 + headersLength);

      let payload;
      switch (headers.get("content-type")) {
        case "application/json":
          payload = JSON.parse(new TextDecoder().decode(body));
          break;
        case "application/octet-stream":
          payload = body;
          break;
        default:
          try {
            payload = JSON.parse(new TextDecoder().decode(body));
          } catch (e) {
            payload = body;
          }
      }

      let type = headers.get("content-location");

      if (options.as === "category") {
        const { component, topic } = parse(type);

        if (type.startsWith("/")) {
          type = `/on/${component}/${topic}`;
        } else {
          type = `on/${component}/${topic}`;
        }
      }

      const ruesEvent = new RuesEvent(type);
      ruesEvent.#headers = headers;
      ruesEvent.#payload = payload;

      return ruesEvent;
    } else if (event instanceof RuesEvent) {
      let type = event.headers.get("content-location");

      if (options.as === "category") {
        const { component, element, topic } = parse(type);

        if (type.startsWith("/")) {
          type = `/on/${component}/${topic}`;
        } else {
          type = `on/${component}/${topic}`;
        }
      }

      const ruesEvent = new RuesEvent(type);
      ruesEvent.#headers = event.headers;
      ruesEvent.#payload = event.payload;

      return ruesEvent;
    }
  }
}

export class Rues extends EventTarget {
  #url;
  #socket;
  #session;

  constructor(url) {
    super(url);

    if (typeof url === "string") {
      url = new URL(url);
    } else if (!(url instanceof URL)) {
      throw new TypeError(`${url} is not a valid URL.`);
    }

    if (!["http:", "https:"].includes(url.protocol)) {
      throw new TypeError(`${url} is not a http(s) URL.`);
    }

    const { protocol, hostname, port } = url;

    Object.defineProperty(this, "url", {
      value: new URL(`${protocol}//${hostname}` + (port ? `:${port}` : "")),
      writable: false,
      enumerable: true,
    });

    this.#session = Promise.withResolvers();
  }

  get ruskVersion() {
    return "0.8.0";
  }

  get sessionId() {
    return this.#session.promise;
  }

  static connect(url, options = {}) {
    return new Rues(url).connect(options);
  }

  async connect(options = {}) {
    const url = new URL(this.#url);
    url.protocol = protocol[url.protocol];
    url.pathname = "/on";

    const { signal } = options;
    const socket = new WebSocket(url);
    socket.binaryType = "arraybuffer";
    this.#socket = socket;
    socket.onerror = console.error;

    if (signal?.aborted) {
      this.#session.reject(signal.reason);
    } else if (signal) {
      signal.addEventListener("abort", (event) => {
        socket.close();
      });
    }

    await once(socket, "open");
    const event = await once(socket, "message");

    socket.addEventListener("message", this, { signal });

    this.#session.resolve(event.data);

    return this;
  }

  async disconnect() {
    if (this.connected) {
      this.#socket.close();
      await once(this.#socket, "close");
    }

    this.#session = Promise.withResolvers();
  }

  get connected() {
    return this.#socket?.readyState === WebSocket.OPEN;
  }

  get on() {
    const target = Object.create(null);
    target[_rues] = this;

    return new Proxy(target, listenerComponentHandler);
  }

  get once() {
    const target = Object.create(null);
    target[_rues] = this;
    target[_once] = Promise.withResolvers();

    return new Proxy(target, listenerComponentHandler);
  }

  get invoke() {
    const target = Object.create(null);
    target[_rues] = this;

    return new Proxy(target, dispatcherComponentHandler);
  }

  handleEvent(event) {
    if (event instanceof MessageEvent) {
      let ruesEvent = RuesEvent.from(event);
      let ruesCategoryEvent = RuesEvent.from(ruesEvent, { as: "category" });
      this.dispatchEvent(ruesEvent);
      this.dispatchEvent(ruesCategoryEvent);
    }
  }
}
