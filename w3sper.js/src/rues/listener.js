// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const _target = Symbol("listener::target");

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

  await response.body?.cancel();

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
          `Unable to subscribe: Unknown Error ${response.status}: ${response.statusText}`
        );
    }
  }

  return eventURL;
};

const unsubscribe = async (target, topic, options) => {
  const headers = merge(target[_target].options.headers, options.headers);

  /**
   * We don't use all the options, because if the signal has
   * been aborted by a user to remove a listener we still
   * want to unsubscribe from the RUES event.
   */
  options = { ...target[_target].options, headers };

  const eventURL = new URL(target[_target].toURL() + topic);
  const { rues } = target[_target];

  headers.append("rusk-version", rues.version);

  if (rues.connected) {
    headers.append("rusk-session-id", await rues.sessionId);
  }

  const response = await fetch(eventURL, { ...options, method: "DELETE" });

  await response.body?.cancel();

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
          `Unable to unsubscribe: Unknown Error ${response.status}: ${response.statusText}`
        );
    }
  }

  return eventURL;
};

export class ListenerError extends Error {
  constructor(message) {
    const [, name, description] = message.match(
      /([A-Za-z\-]+?[ ]?Error)[: ]? ?(.*)/
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
      this.once = Promise.withResolvers();
    }

    return new Proxy(this, this.#handler);
  }

  #handler = {
    get(target, topic) {
      return async (listener, options = {}) => {
        const { signal: optionsSignal } = options;

        if (optionsSignal?.aborted) {
          return;
        }

        const { rues } = target[_target];

        if (target.once) {
          const eventURL = await subscribe(target, topic, options);
          const listenerController = new AbortController();
          const { signal } = listenerController;
          const handleDisrupt = (event) => {
            target.once.reject(event);
            listenerController.abort();

            // we don't care about handling errors in this case
            unsubscribe(target, topic, options).catch(console.error);
          };

          rues.addEventListener(
            eventURL.pathname,
            (event) => {
              target.once.resolve(event);
              listenerController.abort();
            },
            { signal }
          );
          rues.addEventListener("error", handleDisrupt, { signal });
          rues.addEventListener("disconnect", handleDisrupt, { once: true });

          if (optionsSignal) {
            optionsSignal.addEventListener("abort", handleDisrupt, {
              once: true,
            });
          }

          return target.once.promise;
        } else {
          const eventURL = await subscribe(target, topic, options);

          const handleDisrupt = () => {
            rues.removeEventListener(eventURL.pathname, listener);

            // we don't care about handling errors in this case
            unsubscribe(target, topic, options).catch(console.error);
          };

          rues.addEventListener(eventURL.pathname, listener);
          rues.addEventListener("disconnect", handleDisrupt, { once: true });

          if (optionsSignal) {
            optionsSignal.addEventListener("abort", handleDisrupt, {
              once: true,
            });
          }
        }
      };
    },
  };
}
