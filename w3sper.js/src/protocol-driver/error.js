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
      case 253:
        throw new DriverDeserializeError();
      case 0:
        // Exit Code `0` is a success code
        break;
      default:
        throw new DriverError(`Unknown error code: ${code}`);
    }
  }
}

export class DriverArchiveError extends DriverError {
  constructor() {
    super("Failed to archive the data");
  }
}

export class DriverUnarchiveError extends DriverError {
  constructor() {
    super("Failed to unarchive the buffer");
  }
}

export class DriverDeserializeError extends DriverError {
  constructor() {
    super("Failed to deserialize the buffer");
  }
}
