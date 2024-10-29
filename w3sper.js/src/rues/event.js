// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

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

export class RuesEvent extends Event {
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

      if (options.as === "component") {
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

      if (options.as === "component") {
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
