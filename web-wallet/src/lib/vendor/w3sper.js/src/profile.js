// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../src/protocol-driver/mod.js";
import * as base58 from "./encoders/b58.js";

const _index = Symbol("profile::index");
const _seeder = Symbol("profile::seeder");

class Key {
  #buffer;

  constructor(buffer) {
    this.#buffer = buffer.slice();
  }

  toString() {
    return base58.encode(this.#buffer);
  }

  valueOf() {
    return this.#buffer.slice();
  }

  [Symbol.toPrimitive](hint) {
    if (hint === "number") {
      return this[_index];
    } else if (hint === "string") {
      return this.toString();
    }
    return null;
  }

  get seed() {
    return ProfileGenerator.seedFrom(this);
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
    return ProfileGenerator.seedFrom(this);
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
    profile[_seeder] = this[_seeder];
    profile[_index] = n;
    profile.address[_index] = n;
    profile.account[_index] = n;
    profile.address[_seeder] = this[_seeder];
    profile.account[_seeder] = this[_seeder];

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

  static seedFrom(target) {
    return target[_seeder]?.();
  }
}
