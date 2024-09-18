// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

export class DriverError extends Error {
  constructor(message) {
    super(message);
    this.name = this.constructor.name;
  }
  static from(code) {
    switch (code) {
      case 255:
        throw new DriverArchiveError();
      case 254:
        throw new DriverUnarchiveError();
      case 0:
        return 0;
    }
  }
}

export class DriverArchiveError extends DriverError {
  constructor() {
    super("Failed to serialize the data");
  }
}

export class DriverUnarchiveError extends DriverError {
  constructor() {
    super("Failed to parse the buffer");
  }
}
