// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

class RuesEventOrigin {
  scope;
  id;
  topic;

  constructor(source, options) {
    const [, target, topic] = source.match(/^\/?on\/([^\\]+)\/(.+)/) ?? [];

    const [scope, ...id] = target?.split(":") ?? [];

    if (target) {
      this.scope = scope;
      this.topic = topic;

      if (options?.as !== "component") {
        this.id = id.join(":");
      }
    }

    return Object.freeze(this);
  }

  toString() {
    return `/on/${this.scope}${this.id ? ":" + this.id : ""}/${this.topic}`;
  }
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
    return new RuesEventOrigin(this.headers.get("content-location"));
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
          } catch (_e) {
            payload = body;
          }
      }

      const type = new RuesEventOrigin(
        headers.get("content-location"),
        options,
      ).toString();

      const ruesEvent = new RuesEvent(type);
      ruesEvent.#headers = headers;
      ruesEvent.#payload = payload;

      return ruesEvent;
    } else if (event instanceof RuesEvent) {
      const type = new RuesEventOrigin(
        event.headers.get("content-location"),
        options,
      ).toString();

      const ruesEvent = new RuesEvent(type);
      ruesEvent.#headers = event.headers;
      ruesEvent.#payload = event.payload;

      return ruesEvent;
    }
  }
}
