// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const _target = Symbol("callable::target");

function merge(headers1, headers2) {
  headers1 = new Headers(headers1);
  headers2 = new Headers(headers2);

  for (const [key, value] of headers2.entries()) {
    headers1.set(key, value);
  }

  return headers1;
}

const subscribe = async (target, topic, options) => {
  const headers = merge(target[_target].options.headers, options.headers);
  options = { ...target[_target].options, ...options, headers };

  const { signal } = options;

  if (signal?.aborted) {
    return;
  }

  const eventURL = new URL(target[_target].toURL() + topic);
  const { rues } = target[_target];

  headers.append("rusk-version", rues.version);

  if (rues.connected) {
    headers.append("rusk-session-id", await rues.sessionId);
  }

  const response = await fetch(eventURL, options);

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

const unsubscribe = async (target, topic, options) => {
  const headers = merge(target[_target].options.headers, options.headers);
  options = { ...target[_target].options, ...options, headers };

  const { signal } = options;

  if (signal?.aborted) {
    return;
  }

  const eventURL = new URL(target[_target].toURL() + topic);
  const { rues } = target[_target];

  headers.append("rusk-version", rues.version);

  if (rues.connected) {
    headers.append("rusk-session-id", await rues.sessionId);
  }

  const response = await fetch(eventURL, { ...options, method: "DELETE" });

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

export class ListenerError extends Error {
  constructor(message) {
    const [, name, description] = message.match(
      /([A-Za-z\-]+?[ ]?Error)[: ]? ?(.*)/,
    ) ?? [, "ListenerError", message];

    super(description);
    this.name = name;
  }
}

export class ListenerProxy {
  once;

  constructor(target, options = {}) {
    this[_target] = target;

    if (options.once) {
      this.once = Promise.withResolver();
    }

    return new Proxy(this, this.#handler);
  }

  #handler = {
    get(target, topic) {
      return async (listener, options = {}) => {
        const eventURL = await subscribe(target, topic, options);
        const { rues } = target[_target];

        if (target.once) {
          rues.addEventListener(eventURL.pathname, target.once.resolve, {
            once: true,
          });
          return target.once.promise;
        } else {
          rues.addEventListener(eventURL.pathname, listener);
        }

        if (signal) {
          signal.addEventListener("abort", async (event) => {
            await unsubscribe(target, topic, options);
          });
        }
      };
    },
  };
}
