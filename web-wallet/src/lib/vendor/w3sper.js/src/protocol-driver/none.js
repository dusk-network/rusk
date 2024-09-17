// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const NullTarget = Object.create(null);

const _none = Symbol("none::instance");

const DEFAULT_MESSAGE =
  "The object is uninitialized. Please initialize it before use.";
const X = Symbol("X");

export const none = function (strings, ...values) {
  return new Proxy(
    NullTarget,
    new Proxy(NullTarget, {
      get(_target, prop) {
        if (prop === "getPrototypeOf") {
          return undefined;
        }

        return (_target, prop) => {
          if (prop === _none) {
            return true;
          }

          const msg = strings
            ? strings.map((str, i) => str + (values[i] ?? "")).join("")
            : DEFAULT_MESSAGE;

          throw new TypeError(msg);
        };
      },
    })
  );
};

Object.defineProperty(none, Symbol.hasInstance, {
  value: (instance) => _none in instance,
  writable: false,
});
