// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/**
 * Class representing a gas configuration.
 *
 * @class Gas
 *
 * @property {number} limit The gas limit of the wallet. If a value less than or equal to 0, null, or undefined is passed, it defaults to 2,900,000,000.
 * @property {number} price The gas price of the wallet. If a value less than or equal to 0, null, or undefined is passed, it defaults to 1.
 *
 * @example
 * const gas = new Gas({ limit: 3000000, price: 2 });
 * // gas.limit = 3000000
 * // gas.price = 2
 *
 * const defaultGas = new Gas();
 * // defaultGas.limit = 2900000000
 * // defaultGas.price = 1
 */
export class Gas {
  static DEFAULT_LIMIT = 2_900_000_000;
  static DEFAULT_PRICE = 1;

  limit = NaN;
  price = NaN;

  // Passing null/undefined/0 or negative values will set the default value for price and limit
  constructor({ limit, price } = {}) {
    this.limit = Math.max(limit, 0) || Gas.DEFAULT_LIMIT;
    this.price = Math.max(price, 0) || Gas.DEFAULT_PRICE;

    Object.freeze(this);
  }
}
