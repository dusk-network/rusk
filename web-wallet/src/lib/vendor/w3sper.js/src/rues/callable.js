// @ts-nocheck
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

export class CallError extends Error {
  constructor(message) {
    const [, name, description] = message.match(
      /([A-Za-z\-]+?[ ]?Error)[: ]? ?(.*)/
    ) ?? [, "CallError", message];

    super(description);
    this.name = name;
  }
}

export class CallableProxy {
  constructor(target) {
    this[_target] = target;

    return new Proxy(this, this.#handler);
  }

  #handler = {
    get(target, topic) {
      return async (body, options = {}) => {
        const headers = merge(target[_target].options.headers, options.headers);
        options = { ...target[_target].options, ...options, headers };

        const { signal } = options;

        if (signal?.aborted) {
          return;
        }

        const eventURL = new URL(target[_target].toURL() + topic);

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
            throw new CallError(body);
          }
        }

        return response;
      };
    },
  };
}
