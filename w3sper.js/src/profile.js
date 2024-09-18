// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../src/protocol-driver.js";
import * as base58 from "./b58.js";

const _index = Symbol("profile::index");
const _seeder = Symbol("profile::seeder");

class Key {
  #buffer;

  constructor(buffer) {
    this.#buffer = buffer;
  }

  toString() {
    return base58.encode(this.#buffer);
  }
}

export class Profile {
  #buffer;
  #address;
  #account;
  [_index] = -1;

  constructor(buffer) {
    this.#buffer = buffer;
    this.#address = new Key(this.#buffer.subarray(0, 64));
    this.#account = new Key(this.#buffer.subarray(64, 64 + 96));
  }

  get account() {
    return this.#account;
  }

  get address() {
    return this.#address;
  }

  get seed() {
    return this[_seeder]?.();
  }

  sameSourceOf(profile) {
    return profile[_seeder] === this[_seeder];
  }

  [Symbol.toPrimitive](hint) {
    if (hint === "number") {
      return this[_index];
    }
    return null;
  }

  balance(type, source) {
    switch (type) {
      case "account":
        throw new Error("Not implemented yet");
      case "address":
        return ProtocolDriver.balance(this, source);
        break;
      default:
        throw new Error("Unknown account type");
    }
  }
}

export class ProfileGenerator {
  [_seeder];
  #profiles = [];

  constructor(seeder) {
    this[_seeder] = seeder;
  }

  async #nth(n) {
    const seed = await this[_seeder]();

    const buffer = await ProtocolDriver.generateProfile(seed, n);

    const profile = new Profile(buffer);
    profile[_index] = n;
    profile[_seeder] = this[_seeder];

    return profile;
  }

  next() {
    // Generate the next profile, skipping the default profile
    // in case it hasn't been generated yet
    const index = this.#profiles.length || 1;
    const profile = this.#nth(index);

    this.#profiles[index] = profile;

    return profile;
  }

  get default() {
    if (typeof this.#profiles[0] === "undefined") {
      this.#profiles[0] = this.#nth(0);
    }

    return this.#profiles[0];
  }

  indexOf(profile) {
    return profile[_index];
  }

  at(index) {
    return this.#profiles.at(index);
  }

  get length() {
    return this.#profiles.length;
  }

  static typeOf(value) {
    const bytes = base58.decode(value);
    const length = bytes?.length;

    if (length === 64) {
      return "address";
    }
    if (length === 96) {
      return "account";
    }
    return "undefined";
  }
}