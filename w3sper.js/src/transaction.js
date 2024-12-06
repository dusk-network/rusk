// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export const TRANSFER =
  "0100000000000000000000000000000000000000000000000000000000000000";

import { AddressSyncer } from "./network/syncer/address.js";
import * as ProtocolDriver from "./protocol-driver/mod.js";
import { Profile, ProfileGenerator } from "./profile.js";
import * as base58 from "./encoders/b58.js";
import { Gas } from "./gas.js";

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
    const identifier = String(value);
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

  memo(value) {
    this[_attributes].memo = value;
    return this;
  }

  async build(network) {
    const sender = this.bookentry.profile;
    const { attributes } = this;
    const { to, amount: transfer_value, memo: data, gas } = attributes;

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
    } else if (typeof this.bookentry?.info.balance === "function") {
      ({ nonce } = await this.bookentry.info.balance("account"));
    }

    nonce += 1n;

    const [buffer, hash] = await ProtocolDriver.moonlight({
      sender,
      receiver,
      transfer_value,
      deposit: 0n,
      gas_limit: gas.limit,
      gas_price: gas.price,
      nonce,
      chainId,
      data,
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
    const [tx, circuits] = await ProtocolDriver.phoenix({
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
    const [tx, circuits] = await ProtocolDriver.unshield({
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
    const { profile } = this.bookentry;

    // Get the chain id from the network
    const { chainId } = await network.node.info;

    // Obtain the nonce
    let { nonce } = await this.bookentry.info.balance("account");

    nonce += 1n;

    const [buffer, hash] = await ProtocolDriver.shield({
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

export class StakeTransfer extends BasicTransfer {
  constructor(from, options = {}) {
    super(from);
    this[_attributes].topup = Boolean(options.topup) || false;
  }

  async build(network) {
    const { attributes } = this;
    const { amount: stake_value, gas, topup: isTopup } = attributes;
    const { profile, bookkeeper } = this.bookentry;

    const minimumStake = await bookkeeper.minimumStake;

    if (!isTopup && stake_value < minimumStake) {
      throw new RangeError(
        `Stake amount must be greater or equal than ${minimumStake}`,
      );
    }

    // Get the chain id from the network
    const { chainId } = await network.node.info;

    // Obtain the infos
    let { nonce } = await this.bookentry.info.balance("account");
    const stakeInfo = await this.bookentry.info.stake();
    const hasStake = stakeInfo.amount !== null;

    if (hasStake && !isTopup) {
      throw new Error(
        "Stake already exists. Use `topup` to add to the current stake",
      );
    } else if (!hasStake && isTopup) {
      throw new Error("No stake to topup. Use `stake` to create a new stake");
    }

    nonce += 1n;

    const [buffer, hash] = await ProtocolDriver.stake({
      profile,
      stake_value,
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

export class UnstakeTransfer extends BasicTransfer {
  constructor(from) {
    super(from);
  }

  async build(network) {
    const { attributes } = this;
    const { gas, amount: unstake_amount } = attributes;
    const { profile } = this.bookentry;

    // Get the chain id from the network
    const { chainId } = await network.node.info;

    // Obtain the nonces
    let { nonce } = await this.bookentry.info.balance("account");

    // Obtain the staked amount
    const { amount } = await this.bookentry.info.stake();

    const minimumStake = await this.bookentry.bookkeeper.minimumStake;

    nonce += 1n;

    const unstake_value =
      typeof unstake_amount === "bigint" && unstake_amount < amount.total
        ? unstake_amount
        : amount.total;

    const remainingStake = amount.total - unstake_value;

    if (remainingStake > 0n && remainingStake < minimumStake) {
      throw new RangeError(
        `Remaining stake must be greater or equal than ${minimumStake}`,
      );
    }

    const [buffer, hash] = await ProtocolDriver.unstake({
      profile,
      unstake_value,
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

export class WithdrawStakeRewardTransfer extends BasicTransfer {
  constructor(from) {
    super(from);
  }

  async build(network) {
    const { attributes } = this;
    const { amount: reward_amount, gas } = attributes;
    const { profile } = this.bookentry;

    // Get the chain id from the network
    const { chainId } = await network.node.info;

    // Obtain the nonces
    let { nonce } = await this.bookentry.info.balance("account");

    // Obtain the staked amount
    const { reward } = await this.bookentry.info.stake();

    if (!reward) {
      throw new Error(`No stake available to withdraw the reward from`);
    } else if (reward_amount > reward) {
      throw new RangeError(
        `The withdrawn reward amount must be less or equal to ${reward}`,
      );
    } else if (!reward_amount) {
      throw new RangeError(
        `Can't withdraw an empty reward amount. I mean, you could, but it would be pointless.`,
      );
    }

    nonce += 1n;

    const [buffer, hash] = await ProtocolDriver.withdraw({
      profile,
      reward_amount,
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
