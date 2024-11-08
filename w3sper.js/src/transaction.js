// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export const TRANSFER =
  "0100000000000000000000000000000000000000000000000000000000000000";

import { AddressSyncer } from "./network/syncer/address.js";
import { Gas } from "./network/gas.js";
import * as ProtocolDriver from "./protocol-driver/mod.js";
import { ProfileGenerator, Profile } from "./profile.js";
import * as base58 from "./b58.js";

const _attributes = Symbol("builder::attributes");

class BasicTransfer {
  [_attributes];

  constructor(from) {
    this[_attributes] = Object.create(null);

    const value = from instanceof Profile ? { profile: from } : from;

    Object.defineProperty(this, "bookentry", {
      value,
    });

    this[_attributes].gas = new Gas();
  }

  get attributes() {
    return { ...this[_attributes] };
  }

  amount(value) {
    this[_attributes].amount = value;
    return this;
  }

  gas(value) {
    this[_attributes].gas = new Gas(value);
    return this;
  }
}

export class Transfer extends BasicTransfer {
  constructor(from) {
    super(from);
  }

  to(value) {
    let builder;
    let identifier = String(value);
    switch (ProfileGenerator.typeOf(identifier)) {
      case "account":
        builder = new AccountTransfer(this.bookentry);
        break;
      case "address":
        builder = new AddressTransfer(this.bookentry);
        break;
      default:
        throw new TypeError("Invalid identifier");
    }
    this[_attributes].to = identifier;
    builder[_attributes] = this.attributes;

    return builder;
  }
}

class AccountTransfer extends Transfer {
  constructor(from) {
    super(from);
  }

  chain(value) {
    this[_attributes].chain = value;
    return this;
  }

  nonce(value) {
    this[_attributes].nonce = value;
    return this;
  }

  async build(network) {
    const sender = this.bookentry.profile;
    const { attributes } = this;
    const { to, amount: transfer_value, gas } = attributes;

    const receiver = base58.decode(to);

    // Obtain the chain id
    let chainId;
    if (!isNaN(+attributes.chain)) {
      chainId = +attributes.chain;
    } else if (network) {
      ({ chainId } = await network.node.info);
    } else {
      throw new Error("Chain ID is required.");
    }

    // Obtain the nonce
    let nonce;
    if ("nonce" in attributes) {
      ({ nonce } = attributes);
    } else if (typeof this.bookentry?.balance === "function") {
      ({ nonce } = await this.bookentry.balance("account"));
    }

    nonce += 1n;

    let [buffer, hash] = await ProtocolDriver.moonlight({
      sender,
      receiver,
      transfer_value,
      deposit: 0n,
      gas_limit: gas.limit,
      gas_price: gas.price,
      nonce,
      chainId,
      data: null,
    });

    return Object.freeze({
      buffer,
      hash,
      nonce,
    });
  }
}

class AddressTransfer extends Transfer {
  constructor(from) {
    super(from);
  }

  obfuscated() {
    this[_attributes].obfuscated = true;
    return this;
  }

  async build(network) {
    const { attributes } = this;
    const {
      to,
      amount: transfer_value,
      obfuscated: obfuscated_transaction,
      gas,
    } = attributes;
    const sender = this.bookentry.profile;
    const receiver = base58.decode(to);

    const { bookkeeper } = this.bookentry;

    // Pick notes to spend from the treasury
    const picked = await bookkeeper.pick(
      sender.address,
      transfer_value + gas.total,
    );

    const syncer = new AddressSyncer(network);

    // Fetch the openings from the network for the picked notes
    const openings = (await syncer.openings(picked)).map((opening) => {
      return new Uint8Array(opening.slice(0));
    });

    // Fetch the root
    const root = await syncer.root;

    const inputs = picked.values();
    const nullifiers = [...picked.keys()];

    // Get the chain id from the network
    const { chainId } = await network.node.info;

    // Create the unproven transaction
    let [tx, circuits] = await ProtocolDriver.phoenix({
      sender,
      receiver,
      inputs,
      openings,
      root,
      transfer_value,
      obfuscated_transaction,
      deposit: 0n,
      gas_limit: gas.limit,
      gas_price: gas.price,
      chainId,
      data: null,
    });

    // Attempt to prove the transaction
    const proof = await network.prove(circuits);

    // Transform the unproven transaction into a proven transaction
    const [buffer, hash] = await ProtocolDriver.intoProven(tx, proof);

    return Object.freeze({
      buffer,
      hash,
      nullifiers,
    });
  }
}

export class UnshieldTransfer extends BasicTransfer {
  constructor(from) {
    super(from);
  }

  async build(network) {
    const { attributes } = this;
    const { amount: allocate_value, gas } = attributes;
    const { profile, bookkeeper } = this.bookentry;

    // Pick notes to spend from the treasury
    const picked = await bookkeeper.pick(
      profile.address,
      allocate_value + gas.total,
    );

    const syncer = new AddressSyncer(network);

    // Fetch the openings from the network for the picked notes
    const openings = (await syncer.openings(picked)).map((opening) => {
      return new Uint8Array(opening.slice(0));
    });

    // Fetch the root
    const root = await syncer.root;

    const inputs = picked.values();
    const nullifiers = [...picked.keys()];

    // Get the chain id from the network
    const { chainId } = await network.node.info;

    // Create the unproven transaction
    let [tx, circuits] = await ProtocolDriver.unshield({
      profile,
      inputs,
      openings,
      nullifiers,
      root,
      allocate_value,
      gas_limit: gas.limit,
      gas_price: gas.price,
      chainId,
    });

    // Attempt to prove the transaction
    const proof = await network.prove(circuits);

    // Transform the unproven transaction into a proven transaction
    const [buffer, hash] = await ProtocolDriver.intoProven(tx, proof);

    return Object.freeze({
      buffer,
      hash,
      nullifiers,
    });
  }
}

export class ShieldTransfer extends BasicTransfer {
  constructor(from) {
    super(from);
  }

  async build(network) {
    const { attributes } = this;
    const { amount: allocate_value, gas } = attributes;
    const { profile, bookkeeper } = this.bookentry;

    // Get the chain id from the network
    const { chainId } = await network.node.info;

    // Obtain the nonce
    let { nonce } = await this.bookentry.balance("account");

    nonce += 1n;

    let [buffer, hash] = await ProtocolDriver.shield({
      profile,
      allocate_value,
      gas_limit: gas.limit,
      gas_price: gas.price,
      nonce,
      chainId,
    });

    return Object.freeze({
      buffer,
      hash,
      nonce,
    });
  }
}
